// 새롬 확장. 기능은 모두 `saeromc --lsp` 가 한다.
const vscode = require('vscode');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { LanguageClient } = require('vscode-languageclient/node');

let client;

// 설정 > ~/.local/bin·작업 폴더의 빌드 중 가장 새것 > PATH
function compiler() {
  const given = vscode.workspace.getConfiguration('saerom').get('compiler');
  if (given) return given;
  const folders = (vscode.workspace.workspaceFolders || []).map((f) => f.uri.fsPath);
  const found = [
    path.join(os.homedir(), '.local/bin/saeromc'),
    ...folders.flatMap((f) => ['release', 'debug'].map((m) => path.join(f, 'target', m, 'saeromc'))),
  ]
    .filter((file) => fs.existsSync(file))
    .sort((a, b) => fs.statSync(b).mtimeMs - fs.statSync(a).mtimeMs);
  return found[0] || 'saeromc';
}

function start() {
  const command = compiler();
  client = new LanguageClient(
    'saerom',
    '새롬',
    { command, args: ['--lsp'] },
    { documentSelector: [{ scheme: 'file', language: 'saerom' }] },
  );
  return client.start().catch(() => {
    vscode.window.showErrorMessage(`saeromc를 실행할 수 없음: ${command}`);
  });
}

async function run() {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== 'saerom') return;
  await editor.document.save();
  const source = editor.document.uri.fsPath;
  const output = path.join(os.tmpdir(), 'saerom-' + path.basename(source, '.sr'));
  const terminal =
    vscode.window.terminals.find((t) => t.name === '새롬') ||
    vscode.window.createTerminal({ name: '새롬', cwd: path.dirname(source) });
  terminal.show(true);
  const quote = (text) => `'${text.replace(/'/g, `'\\''`)}'`;
  terminal.sendText(`${quote(compiler())} ${quote(source)} -o ${quote(output)} && ${quote(output)}`);
}

function activate(context) {
  context.subscriptions.push(
    vscode.commands.registerCommand('saerom.run', run),
    vscode.commands.registerCommand('saerom.restart', async () => {
      if (client) await client.stop();
      await start();
    }),
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (!event.affectsConfiguration('saerom.compiler')) return;
      if (client) await client.stop();
      await start();
    }),
  );
  return start();
}

function deactivate() {
  return client && client.stop();
}

module.exports = { activate, deactivate };
