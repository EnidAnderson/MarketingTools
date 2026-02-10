import os
from playwright.sync_api import sync_playwright
from src.config import PROJECT_ROOT

def take_screenshot(url: str, output_path: str) -> str:
    """
    Captures a screenshot of the given URL and saves it to the specified path.
    Returns the path to the saved screenshot.
    """
    print(f"Taking screenshot of {url}...")
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page()
        page.goto(url)
        page.screenshot(path=output_path, full_page=True)
        browser.close()
    print(f"Screenshot saved to {output_path}")
    return output_path

if __name__ == "__main__":
    # Example usage
    # Ensure Playwright browsers are installed: playwright install
    output_dir = os.path.join(PROJECT_ROOT, "screenshots") # Modified
    os.makedirs(output_dir, exist_ok=True)
    screenshot_path = take_screenshot(
        url="https://www.naturesdietpet.com",
        output_path=os.path.join(output_dir, "homepage.png")
    )
    print(f"Generated screenshot path: {screenshot_path}")
