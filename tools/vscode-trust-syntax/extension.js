const path = require("path")
const fs = require("fs")
const { promisify } = require("util")
const { execFile } = require("child_process")
const vscode = require("vscode")
const { LanguageClient, TransportKind } = require("vscode-languageclient/node")

const execFileAsync = promisify(execFile)
let client
const lintInFlight = new Set()
let lintDiagnostics

function resolveWorkspaceRoot() {
  const ws = vscode.workspace.workspaceFolders
  if (ws && ws.length > 0) {
    return ws[0].uri.fsPath
  }
  return process.cwd()
}

function findTrustRepoRoot(startDir) {
  let dir = startDir
  while (true) {
    const manifest = path.join(dir, "crates", "trusty-lsp", "Cargo.toml")
    if (fs.existsSync(manifest)) {
      return dir
    }
    const parent = path.dirname(dir)
    if (parent === dir) {
      return null
    }
    dir = parent
  }
}

function makeServerOptions() {
  const cfg = vscode.workspace.getConfiguration("trust.languageServer")
  const command = cfg.get("command", "cargo")
  let args = cfg.get("args", ["run", "--manifest-path", "crates/trusty-lsp/Cargo.toml", "--", "--stdio"])
  const configuredCwd = cfg.get("cwd", "").trim()
  const workspaceRoot = resolveWorkspaceRoot()
  const inferredRepoRoot = findTrustRepoRoot(workspaceRoot)
  const cwd = configuredCwd || inferredRepoRoot || workspaceRoot

  if (command === "cargo" && Array.isArray(args)) {
    const stdioIdx = args.indexOf("--stdio")
    const sepIdx = args.indexOf("--")
    if (stdioIdx >= 0 && (sepIdx < 0 || sepIdx > stdioIdx)) {
      args = [...args]
      args.splice(stdioIdx, 0, "--")
    }
  }

  const maybeManifest = path.join(cwd, "crates", "trusty-lsp", "Cargo.toml")
  if (command === "cargo" && !fs.existsSync(maybeManifest)) {
    vscode.window.showWarningMessage(
      "TRUST LSP: crates/trusty-lsp/Cargo.toml introuvable dans le workspace courant. " +
      "Configure trust.languageServer.cwd ou trust.languageServer.command."
    )
  }

  return {
    command,
    args,
    options: { cwd },
    transport: TransportKind.stdio
  }
}

function makeClientOptions() {
  return {
    documentSelector: [
      { scheme: "file", language: "trust" },
      { scheme: "untitled", language: "trust" }
    ],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.trs")
    }
  }
}

function commandArgsWithFile(args, filePath) {
  return args.map(a => String(a).replaceAll("${file}", filePath))
}

function workspaceSettingsPath() {
  const root = resolveWorkspaceRoot()
  return path.join(root, ".vscode", "settings.json")
}

function extractByteSpan(text) {
  const match =
    text.match(/\((\d+)\.\.(\d+),\s*TS\d+\)/) ||
    text.match(/\((\d+)\.\.(\d+)\)/) ||
    text.match(/(\d+)\.\.(\d+)/)
  if (!match) {
    return null
  }
  const start = Number(match[1])
  const end = Number(match[2])
  if (!Number.isFinite(start) || !Number.isFinite(end) || start < 0) {
    return null
  }
  return { start, end: Math.max(end, start + 1) }
}

function utf8BytesToUtf16Offset(text, targetBytes) {
  const allBytes = Buffer.byteLength(text, "utf8")
  const clamped = Math.max(0, Math.min(targetBytes, allBytes))
  return Buffer.from(text, "utf8").subarray(0, clamped).toString("utf8").length
}

function rangeFromByteSpan(document, span) {
  if (!span) {
    return null
  }
  const text = document.getText()
  const startOffset = utf8BytesToUtf16Offset(text, span.start)
  const endOffset = utf8BytesToUtf16Offset(text, span.end)
  const start = document.positionAt(startOffset)
  const end = document.positionAt(Math.max(endOffset, startOffset + 1))
  return new vscode.Range(start, end)
}

async function ensureWorkspaceSettings() {
  const cfg = vscode.workspace.getConfiguration("trust")
  const autoWrite = cfg.get("setup.writeWorkspaceSettingsOnActivate", false)
  if (!autoWrite) {
    return
  }

  const settingsPath = workspaceSettingsPath()
  const settingsDir = path.dirname(settingsPath)
  await fs.promises.mkdir(settingsDir, { recursive: true })

  let current = {}
  if (fs.existsSync(settingsPath)) {
    try {
      current = JSON.parse(await fs.promises.readFile(settingsPath, "utf8"))
    } catch (_) {
      // Keep existing file untouched if it's not valid JSON.
      return
    }
  }

  const next = { ...current }
  const workspaceRoot = resolveWorkspaceRoot()
  const inferredRepoRoot = findTrustRepoRoot(workspaceRoot)
  next["trust.languageServer.cwd"] = next["trust.languageServer.cwd"] || inferredRepoRoot || workspaceRoot
  next["trust.languageServer.command"] = next["trust.languageServer.command"] || "cargo"
  next["trust.languageServer.args"] = next["trust.languageServer.args"] || [
    "run",
    "--manifest-path",
    "crates/trusty-lsp/Cargo.toml",
    "--",
    "--stdio"
  ]
  next["trust.format.command"] = next["trust.format.command"] || "cargo"
  next["trust.format.args"] = next["trust.format.args"] || [
    "run",
    "--manifest-path",
    "crates/trusty-cli/Cargo.toml",
    "--",
    "format",
    "${file}"
  ]
  next["trust.lint.command"] = next["trust.lint.command"] || "cargo"
  next["trust.lint.args"] = next["trust.lint.args"] || [
    "run",
    "--manifest-path",
    "crates/trusty-cli/Cargo.toml",
    "--",
    "check",
    "${file}"
  ]
  next["trust.lint.onSave"] = next["trust.lint.onSave"] !== undefined ? next["trust.lint.onSave"] : true
  next["editor.formatOnSave"] = next["editor.formatOnSave"] !== undefined ? next["editor.formatOnSave"] : true
  const lang = next["[trust]"] || {}
  if (lang["editor.formatOnSave"] === undefined) {
    lang["editor.formatOnSave"] = true
  }
  next["[trust]"] = lang

  if (JSON.stringify(current) !== JSON.stringify(next)) {
    await fs.promises.writeFile(settingsPath, `${JSON.stringify(next, null, 2)}\n`, "utf8")
  }
}

function makeFormatterProvider() {
  return {
    async provideDocumentFormattingEdits(document) {
      const filePath = document.uri.fsPath
      const cfg = vscode.workspace.getConfiguration("trust")
      const command = cfg.get("format.command", "cargo")
      const args = commandArgsWithFile(
        cfg.get("format.args", ["run", "--manifest-path", "crates/trusty-cli/Cargo.toml", "--", "format", "${file}"]),
        filePath
      )
      const cwd = vscode.workspace.getConfiguration("trust.languageServer").get("cwd", "").trim() || resolveWorkspaceRoot()

      const tmpDir = path.join(cwd, ".trusty", "tmp")
      await fs.promises.mkdir(tmpDir, { recursive: true })
      const tmpFile = path.join(tmpDir, `format-${Date.now()}-${Math.random().toString(36).slice(2)}.trs`)

      try {
        await fs.promises.writeFile(tmpFile, document.getText(), "utf8")
        const tmpArgs = commandArgsWithFile(args, tmpFile)
        await execFileAsync(command, tmpArgs, { cwd })
        const formatted = await fs.promises.readFile(tmpFile, "utf8")

        const fullRange = new vscode.Range(
          new vscode.Position(0, 0),
          document.lineAt(document.lineCount - 1).range.end
        )
        return [vscode.TextEdit.replace(fullRange, formatted)]
      } catch (err) {
        vscode.window.showErrorMessage(`TRUST format failed: ${err.message || String(err)}`)
        return []
      } finally {
        fs.promises.unlink(tmpFile).catch(() => {})
      }
    }
  }
}

function registerLintOnSave(context) {
  const d = vscode.workspace.onDidSaveTextDocument(async (document) => {
    if (document.languageId !== "trust") {
      return
    }
    const cfg = vscode.workspace.getConfiguration("trust")
    if (!cfg.get("lint.onSave", true)) {
      return
    }
    const filePath = document.uri.fsPath
    if (lintInFlight.has(filePath)) {
      return
    }
    lintInFlight.add(filePath)
    try {
      const command = cfg.get("lint.command", "cargo")
      const args = commandArgsWithFile(
        cfg.get("lint.args", ["run", "--manifest-path", "crates/trusty-cli/Cargo.toml", "--", "check", "${file}"]),
        filePath
      )
      const cwd = vscode.workspace.getConfiguration("trust.languageServer").get("cwd", "").trim() || resolveWorkspaceRoot()
      await execFileAsync(command, args, { cwd })
      lintDiagnostics.delete(document.uri)
    } catch (err) {
      const stderr = err.stderr ? String(err.stderr).trim() : ""
      const msg = stderr || err.message || String(err)
      const clean = msg.replace(/\x1b\[[0-9;]*m/g, "")
      const fullEnd = document.lineCount > 0
        ? document.lineAt(document.lineCount - 1).range.end
        : new vscode.Position(0, 1)
      const range = rangeFromByteSpan(document, extractByteSpan(clean))
        || new vscode.Range(new vscode.Position(0, 0), fullEnd)
      const diag = new vscode.Diagnostic(
        range,
        `TRUST lint failed: ${clean}`,
        vscode.DiagnosticSeverity.Error
      )
      diag.source = "trusty-lint"
      lintDiagnostics.set(document.uri, [diag])
      vscode.window.showErrorMessage(`TRUST lint failed: ${clean}`)
    } finally {
      lintInFlight.delete(filePath)
    }
  })
  context.subscriptions.push(d)
}

async function startClient() {
  if (client) {
    return
  }
  client = new LanguageClient(
    "trustyLsp",
    "TRUST Language Server",
    makeServerOptions(),
    makeClientOptions()
  )
  await client.start()
}

async function stopClient() {
  if (!client) {
    return
  }
  const c = client
  client = undefined
  await c.stop()
}

async function restartClient() {
  await stopClient()
  await startClient()
  vscode.window.showInformationMessage("TRUST Language Server restarted.")
}

async function activate(context) {
  lintDiagnostics = vscode.languages.createDiagnosticCollection("trusty-lint")
  context.subscriptions.push(
    vscode.commands.registerCommand("trust.restartLanguageServer", restartClient),
    vscode.commands.registerCommand("trust.setupWorkspace", async () => {
      await ensureWorkspaceSettings()
      vscode.window.showInformationMessage("TRUST workspace settings updated.")
    }),
    vscode.languages.registerDocumentFormattingEditProvider({ language: "trust", scheme: "file" }, makeFormatterProvider()),
    lintDiagnostics
  )
  await ensureWorkspaceSettings()
  registerLintOnSave(context)
  await startClient()
}

async function deactivate() {
  await stopClient()
}

module.exports = {
  activate,
  deactivate
}
