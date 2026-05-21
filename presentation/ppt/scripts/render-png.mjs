// Render every .slide of the j-cli HTML deck to a separate high-res PNG
import puppeteer from 'puppeteer';
import { mkdir } from 'fs/promises';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PPT_DIR = path.resolve(__dirname, '..');

const HTML_PATH = process.argv[2] || path.join(PPT_DIR, 'index.html');
const OUT_DIR   = process.argv[3] || path.join(PPT_DIR, 'ppt-png');

const fileUrl = 'file://' + path.resolve(HTML_PATH);

// 16:9 — use a smaller CSS viewport so content fills the frame like it does in
// a real browser (macOS Retina default viewport ≈ 1440-1470 CSS px wide).
// deviceScaleFactor is increased proportionally so the output PNG is still
// high-resolution (≈ 2880 x 1620, > 1080p).
const W = 1440, H = 810;
const SCALE = 2;

async function main() {
  await mkdir(OUT_DIR, { recursive: true });

  const browser = await puppeteer.launch({
    headless: 'new',
    args: ['--no-sandbox', '--disable-web-security'],
    defaultViewport: { width: W, height: H, deviceScaleFactor: SCALE },
  });

  const page = await browser.newPage();
  page.on('pageerror', err => console.log('  [browser]', err.message));

  console.log(`Loading ${fileUrl}`);
  await page.goto(fileUrl, { waitUntil: 'networkidle0', timeout: 60000 });
  await page.evaluate(() => document.fonts.ready);
  await new Promise(r => setTimeout(r, 1000));

  // Disable slide CSS transitions so screenshot reflects current state immediately
  await page.addStyleTag({
    content: `.deck > section.slide { transition: none !important; }`,
  });

  const total = await page.evaluate(() =>
    document.querySelectorAll('.deck > section.slide').length
  );
  console.log(`Found ${total} slides, rendering each at ${W}x${H} (${SCALE}x DPI = ${W*SCALE}x${H*SCALE}px output)...`);

  for (let i = 1; i <= total; i++) {
    // Directly toggle .is-active instead of relying on hash + runtime listener
    // (hash-based dispatch has subtle timing issues during rapid switches)
    await page.evaluate((idx) => {
      const slides = document.querySelectorAll('.deck > section.slide');
      slides.forEach((s, j) => {
        s.classList.toggle('is-active', j === idx - 1);
        s.classList.remove('is-prev');
      });
      // Also update hash for runtime state consistency
      window.location.hash = '#/' + idx;
    }, i);
    // Wait for the target slide to be visibly active
    await page.waitForFunction((idx) => {
      const target = document.querySelectorAll('.deck > section.slide')[idx - 1];
      return target && target.classList.contains('is-active') &&
             getComputedStyle(target).opacity === '1';
    }, { timeout: 5000 }, i);
    // Small buffer for any image / font swap to settle
    await new Promise(r => setTimeout(r, 100));

    const filename = `slide_${String(i).padStart(2, '0')}.png`;
    const outPath = path.join(OUT_DIR, filename);
    await page.screenshot({
      path: outPath,
      type: 'png',
      omitBackground: false,
      clip: { x: 0, y: 0, width: W, height: H },
    });
    process.stdout.write(`  ✔ ${filename}\n`);
  }

  await browser.close();
  console.log(`Done. ${total} PNGs in ${OUT_DIR}`);
}

main().catch(e => {
  console.error('FATAL:', e);
  process.exit(1);
});
