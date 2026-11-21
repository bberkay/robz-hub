// Arayüzün ekran görüntülerini alır:  npm run screenshots [-- <exe> <çıktı klasörü>]
//
// Robz Hub'ı deneme kipinde (ROBZHUB_DRY_RUN=1) ve geçici bir veri klasörüyle açar, sayfaları sırayla gezer ve
// docs/screenshots altına PNG yazar. Bilgisayardaki kurulumlara dokunmaz; yalnız GitHub'dan sürüm listesini okur.
// WebView2'nin hata ayıklama bağlantı noktasıyla (Chrome DevTools Protocol) konuşur; ek paket gerekmez.
// exe verilmezse önce src-tauri/target/release, yoksa debug derlemesi kullanılır.
'use strict';

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawn } = require('node:child_process');

const root = path.join(__dirname, '..');
const PORT = 9333;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function findExe() {
  if (process.argv[2]) return path.resolve(process.argv[2]);
  for (const profile of ['release', 'debug']) {
    for (const name of ['RobzHub.exe', 'robz-hub.exe']) {
      const file = path.join(root, 'src-tauri', 'target', profile, name);
      if (fs.existsSync(file)) return file;
    }
  }
  throw new Error('Robz Hub derlemesi bulunamadı: önce `npm run build` ya da `cargo build`');
}

async function connect() {
  for (let i = 0; i < 120; i += 1) {
    try {
      const targets = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
      const page = targets.find((t) => t.type === 'page' && /tauri\.localhost/.test(t.url));
      if (page) return new WebSocket(page.webSocketDebuggerUrl);
    } catch {
      // henüz açılmadı
    }
    await sleep(250);
  }
  throw new Error('uygulama penceresi bulunamadı');
}

async function main() {
  const exe = findExe();
  const outDir = path.resolve(process.argv[3] || path.join(root, 'docs', 'screenshots'));
  const dataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'robzhub-shots-'));
  const child = spawn(exe, [], {
    env: {
      ...process.env,
      ROBZHUB_DRY_RUN: '1',
      ROBZHUB_DATA_DIR: dataDir,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PORT}`,
    },
    stdio: 'ignore',
  });

  let exitCode = 0;
  try {
    const ws = await connect();
    await new Promise((resolve, reject) => {
      ws.addEventListener('open', resolve, { once: true });
      ws.addEventListener('error', reject, { once: true });
    });
    let nextId = 0;
    const pending = new Map();
    const logs = [];
    ws.addEventListener('message', (event) => {
      const message = JSON.parse(event.data);
      if (message.id && pending.has(message.id)) {
        pending.get(message.id)(message);
        pending.delete(message.id);
      } else if (message.method === 'Runtime.exceptionThrown') {
        logs.push(`[exception] ${message.params.exceptionDetails.exception?.description || message.params.exceptionDetails.text}`);
      } else if (message.method === 'Log.entryAdded' && message.params.entry.level === 'error') {
        logs.push(`[log.error] ${message.params.entry.text} ${message.params.entry.url || ''}`);
      }
    });
    const send = (method, params = {}) =>
      new Promise((resolve) => {
        nextId += 1;
        pending.set(nextId, resolve);
        ws.send(JSON.stringify({ id: nextId, method, params }));
      });
    const evaluate = async (expression) => {
      const reply = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (reply.result?.exceptionDetails) throw new Error(reply.result.exceptionDetails.exception?.description || 'evaluate hatası');
      return reply.result?.result?.value;
    };
    const waitFor = async (expression, timeout = 20000) => {
      const start = Date.now();
      while (Date.now() - start < timeout) {
        if (await evaluate(expression).catch(() => false)) return true;
        await sleep(200);
      }
      return false;
    };
    const shot = async (name) => {
      await sleep(700);
      const reply = await send('Page.captureScreenshot', { format: 'png' });
      fs.mkdirSync(outDir, { recursive: true });
      fs.writeFileSync(path.join(outDir, `${name}.png`), Buffer.from(reply.result.data, 'base64'));
      console.log(`  ${name}.png`);
    };
    const click = (selector, textMatch) =>
      evaluate(`(() => {
        const nodes = [...document.querySelectorAll(${JSON.stringify(selector)})];
        const node = ${textMatch ? `nodes.find((n) => n.textContent.includes(${JSON.stringify(textMatch)}))` : 'nodes[0]'};
        if (!node) return false;
        node.click();
        return true;
      })()`);

    await send('Runtime.enable');
    await send('Log.enable');
    await send('Page.reload'); // dinleme açıkken baştan yükle: açılış hataları da görünsün

    if (!(await waitFor("document.querySelector('.shell') !== null"))) throw new Error(`arayüz açılmadı\n${logs.join('\n')}`);
    await waitFor("document.querySelectorAll('.feed-item').length > 0", 20000);
    await waitFor("document.querySelector('.topbar .spinner') === null", 20000);
    await evaluate("document.querySelectorAll('.toast .btn-icon').forEach((b) => b.click())");

    console.log(`ekran görüntüleri -> ${outDir}`);
    await evaluate("window.hub.call('setSetting', 'language', 'tr')");
    await sleep(300);
    await shot('01-ana-sayfa');
    await click('.sidebar .nav-item', 'Robz Vision');
    await shot('02-uygulama');
    await click('.tab', 'Sürüm notları');
    await shot('03-surum-notlari');
    await click('.sidebar .nav-item', 'Güncellemeler');
    await shot('04-guncellemeler');
    await click('.sidebar .nav-item', 'Ayarlar');
    await shot('05-ayarlar');
    await evaluate("window.hub.call('setSetting', 'language', 'en')");
    await sleep(300);
    await click('.sidebar .nav-item', 'Home');
    await shot('06-home-en');
    await evaluate("window.hub.call('setSetting', 'language', 'auto')");

    if (logs.length) console.log(`\nkonsol:\n${logs.join('\n')}`);
    ws.close();
  } catch (error) {
    console.error(error.message);
    exitCode = 1;
  } finally {
    child.kill();
    await sleep(800);
    fs.rmSync(dataDir, { recursive: true, force: true });
  }
  process.exit(exitCode);
}

main();
