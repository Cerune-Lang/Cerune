const assert = require('node:assert/strict');
const path = require('node:path');
const {serve} = require('./serve.cjs');

(async () => {
  const [directory, playwrightPath, channel] = process.argv.slice(2);
  if (!directory || !playwrightPath) throw new Error('usage: test.cjs artifacts playwright-path [channel]');
  const {chromium} = require(path.resolve(playwrightPath));
  const server = await serve(directory);
  let browser;
  try {
    browser = await chromium.launch({headless:true, ...(channel ? {channel} : {})});
    const origin = 'http://127.0.0.1:' + server.address().port;
    for (const [pageName, language] of [['index.html','ja'],['index.en.html','en']]) {
      const page = await browser.newPage();
      const errors = [];
      page.on('pageerror', error => errors.push(error.message));
      await page.goto(origin + '/' + pageName);
      await page.waitForSelector('#status[data-state=ready]');
      assert.equal(await page.locator('#position').textContent(), '10');
      await page.locator('#step').click();
      assert.equal(await page.locator('#result').textContent(), '11');
      // Reading metadata does not advance the program.
      const descriptor = JSON.parse(await page.locator('#metadata').textContent());
      assert.equal(descriptor.exports[0].name, 'step');
      await page.locator('#step').click();
      assert.equal(await page.locator('#result').textContent(), '12');
      await page.locator('#limit').fill('4');
      await page.locator('#sum').click();
      assert.equal(await page.locator('#result').textContent(), '10');
      await page.locator('#limit').fill('1.5');
      await page.locator('#sum').click();
      await page.waitForSelector('#status[data-state=rejected]');
      await page.locator('#bad-argument').click();
      assert.match(await page.locator('#diagnostic').textContent(), /^arguments/);
      await page.locator('#step').click();
      assert.equal(await page.locator('#result').textContent(), '13');
      await page.locator('#fail').click();
      await page.waitForSelector('#status[data-state=stopped]');
      assert.match(await page.locator('#diagnostic').textContent(), /runtime-v1 code=conversion-inexact node=\d+ file=2 bytes=\d+\.\.\d+/);
      await page.screenshot({path:path.join(directory,'browser-'+language+'.png'),fullPage:true});
      await page.locator('#step').click();
      assert.match(await page.locator('#diagnostic').textContent(), /^closed/);
      await page.locator('#reload').click();
      await page.waitForSelector('#status[data-state=ready]');
      await page.locator('#step').click();
      assert.equal(await page.locator('#result').textContent(), '11');
      assert.deepEqual(errors, []);
      await page.close();
    }
    console.log('Browser passed: repeated calls, observation, argument rejection, runtime origins, closed handle and fresh instance (ja/en).');
  } finally {
    if (browser) await browser.close();
    await new Promise(resolve => server.close(resolve));
  }
})().catch(error => {console.error(error); process.exitCode=1;});
