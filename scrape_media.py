#!/usr/bin/env python3
"""
Universal scraper for social media video URLs using Playwright + Chromium.
Used as fallback when yt-dlp is blocked by anti-bot protection.

Usage: scrape_media.py <url> <platform>
Output: JSON with title, media items, etc.

Platforms supported:
- instagram: Scrape reel/post video URLs from cdninstagram.com
- facebook: Scrape video URLs from fbcdn.net
- tiktok: Try scraping, but may be blocked by anti-bot
- twitter: Scrape video URLs from video.twimg.com
- pinterest: Scrape video URLs from v.pinimg.com
"""
import sys
import asyncio
import re
import json
import os

# Point Playwright at system-wide browser cache so it works regardless of
# the service user's HOME permissions.
os.environ.setdefault("PLAYWRIGHT_BROWSERS_PATH", "/usr/local/share/ms-playwright")

from playwright.async_api import async_playwright

async def scrape(url, platform):
    async with async_playwright() as p:
        browser = await p.chromium.launch(
            headless=True,
            args=["--no-sandbox", "--disable-bypass"]
        )
        context = await browser.new_context(
            user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            viewport={"width": 1920, "height": 1080},
            java_script_enabled=True,
        )
        page = await context.new_page()

        # Track network responses
        media_responses = []

        def handle_response(response):
            resp_url = response.url
            # Look for video/audio URLs
            if any(ext in resp_url for ext in ['.mp4', '.m3u8', '.mp3']):
                media_responses.append({
                    "url": resp_url,
                    "status": response.status,
                    "content_type": response.headers.get('content-type', ''),
                })

        page.on("response", handle_response)

        try:
            await page.goto(url, timeout=60000)
            await page.wait_for_timeout(15000)
        except Exception as e:
            await browser.close()
            print(json.dumps({"success": False, "error": f"Navigation error: {e}"}))
            return

        # Get page title
        title = await page.title()

        # Get video element info
        video_info = await page.evaluate("""() => {
            const video = document.querySelector('video');
            if (!video) return null;
            return {
                src: video.src,
                duration: video.duration,
                poster: video.poster,
            };
        }""")

        # Filter for real video URLs (not static assets, not ttwstatic)
        seen = set()
        media = []
        for resp in media_responses:
            url_val = resp["url"]
            if url_val in seen:
                continue
            if len(url_val) < 50:
                continue
            # Skip static assets
            if 'ttwstatic' in url_val or 'rsrc.php' in url_val:
                continue
            if 'cdninstagram' not in url_val and 'fbcdn' not in url_val and 'video.twimg' not in url_val and 'pinimg' not in url_val:
                # For non-Instagram/facebook, still accept
                if 'static' in url_val:
                    continue
            seen.add(url_val)

            # Determine content type
            ext = "mp4"
            if '.m3u8' in url_val:
                ext = "m3u8"
            elif '.mp3' in url_val:
                ext = "mp3"
            elif '.mp4' in url_val:
                ext = "mp4"

            media.append({
                "url": url_val,
                "ext": ext,
                "status": resp["status"],
                "content_type": resp["content_type"],
            })

        # Also check for video element src (blob URLs won't work, but worth checking)
        if video_info and video_info.get('src'):
            video_src = video_info['src']
            if not video_src.startswith('blob:'):
                media.append({
                    "url": video_src,
                    "ext": "mp4",
                    "status": 200,
                    "content_type": "video/mp4",
                })

        # If no media found, check page content for URLs
        if not media:
            content = await page.content()
            # Look for video URLs in page source
            content_urls = re.findall(r'https://[^\s"\'<>]+', content)
            for url_val in content_urls:
                if any(ext in url_val for ext in ['.mp4', '.m3u8']) and len(url_val) > 50:
                    if 'ttwstatic' not in url_val and 'rsrc.php' not in url_val:
                        if url_val not in seen:
                            seen.add(url_val)
                            ext = "m3u8" if '.m3u8' in url_val else "mp4"
                            media.append({
                                "url": url_val,
                                "ext": ext,
                                "status": 200,
                                "content_type": f"video/{ext}" if ext != 'm3u8' else 'application/x-mpegURL',
                            })

        result = {
            "success": len(media) > 0,
            "title": title if title else "Unknown",
            "platform": platform,
            "media": media,
            "media_count": len(media),
            "provider": f"playwright-{platform}",
        }

        await browser.close()
        print(json.dumps(result, indent=2))

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print(json.dumps({"success": False, "error": "Usage: scrape_media.py <url> <platform>"}))
        sys.exit(1)

    url = sys.argv[1]
    platform = sys.argv[2]

    asyncio.run(scrape(url, platform))
