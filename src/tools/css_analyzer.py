import requests
from bs4 import BeautifulSoup
from urllib.parse import urljoin
import re

def get_all_css(url: str) -> str:
    """
    Fetches all CSS rules from a given URL, including external stylesheets and inline styles.
    Returns a single string containing all CSS.
    """
    print(f"Analyzing CSS for {url}...")
    combined_css = []
    
    try:
        response = requests.get(url)
        response.raise_for_status()
        soup = BeautifulSoup(response.text, 'html.parser')

        # Extract inline <style> tags
        for style_tag in soup.find_all('style'):
            if style_tag.string:
                combined_css.append(style_tag.string)

        # Extract external stylesheets
        for link_tag in soup.find_all('link', rel='stylesheet', href=True):
            css_url = urljoin(url, link_tag['href'])
            try:
                css_response = requests.get(css_url)
                css_response.raise_for_status()
                combined_css.append(css_response.text)
            except requests.exceptions.RequestException as e:
                print(f"Error fetching external CSS from {css_url}: {e}")
        
    except requests.exceptions.RequestException as e:
        print(f"Error fetching HTML from {url}: {e}")
    
    # Clean up and combine
    # Remove comments and excessive whitespace (basic cleanup)
    final_css = "\n".join(filter(None, combined_css))
    final_css = re.sub(r'/\*.*?\*/', '', final_css, flags=re.DOTALL) # Remove CSS comments
    final_css = re.sub(r'\s{2,}', ' ', final_css) # Replace multiple whitespaces with single space
    final_css = final_css.replace('; ', ';').replace(' {', '{').replace(' }', '}').replace(';}', '}') # Further compact
    
    print(f"Finished CSS analysis for {url}. Total size: {len(final_css)} characters.")
    return final_css

if __name__ == "__main__":
    # Example usage
    target_url = "https://www.naturesdietpet.com"
    css_content = get_all_css(target_url)
    # print(css_content[:1000]) # Print first 1000 characters
    with open("homepage_css.css", "w") as f:
        f.write(css_content)
    print(f"Full CSS saved to homepage_css.css (first 1000 chars printed).")
