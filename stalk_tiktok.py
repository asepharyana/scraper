"""Extract TikTok user-detail JSON (__UNIVERSAL_DATA_FOR_REHYDRATION__) via Playwright.

The direct HTTP path gets a SlardarWAF challenge page; a real browser with JS
executes the challenge and exposes the __UNIVERSAL_DATA_FOR_REHYDRATION__ script.
Usage: python3 stalk_tiktok.py <username>
"""
import sys
import asyncio
import json
import os

os.environ.setdefault("PLAYWRIGHT_BROWSERS_PATH", "/usr/local/share/ms-playwright")

from playwright.async_api import async_playwright


async def main(username):
    url = f"https://www.tiktok.com/@{username}?_t=ZS-8tHANz7ieoS&_r=1"
    async with async_playwright() as p:
        browser = await p.chromium.launch(
            headless=True,
            args=["--no-sandbox", "--disable-bypass", "--disable-blink-features=AutomationControlled"],
        )
        context = await browser.new_context(
            user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            viewport={"width": 1366, "height": 768},
            java_script_enabled=True,
            locale="en-US",
        )
        page = await context.new_page()
        try:
            await page.goto(url, timeout=60000, wait_until="domcontentloaded")
        except Exception as e:
            await browser.close()
            print(json.dumps({"success": False, "error": f"Navigation: {e}"}))
            return

        # Wait for the rehydration script to appear (WAF challenge may take a few seconds)
        found = None
        for _ in range(30):
            try:
                found = await page.evaluate(
                    """() => {
                        const s = document.querySelector('script#__UNIVERSAL_DATA_FOR_REHYDRATION__');
                        return s ? s.textContent : null;
                    }"""
                )
                if found:
                    break
            except Exception:
                pass
            await page.wait_for_timeout(1000)

        await browser.close()

        if not found:
            print(json.dumps({"success": False, "error": "Rehydration data not found (WAF challenge)"}))
            return

        print(json.dumps({"success": True, "data": found}))


if __name__ == "__main__":
    asyncio.run(main(sys.argv[1]))