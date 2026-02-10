import os
import re
import time
from urllib.parse import urljoin, urlparse
import requests
from bs4 import BeautifulSoup
from dotenv import load_dotenv
from langchain_core.prompts import ChatPromptTemplate
from langchain_core.output_parsers import StrOutputParser
from langchain_google_genai import ChatGoogleGenerativeAI
from src.config import PRODUCT_DIR

load_dotenv()

# Ensure GOOGLE_API_KEY is set as an environment variable
if not os.getenv("GOOGLE_API_KEY"):
    raise ValueError("GOOGLE_API_KEY environment variable not set.")

llm = ChatGoogleGenerativeAI(model="gemini-2.5-pro", temperature=0.2)

BASE_URL = "https://naturesdietpet.com"

def sanitize_filename(text):
    """Sanitizes text to be used as a filename."""
    text = re.sub(r'[^\w\s-]', '', text).strip()
    text = re.sub(r'[-\s]+', '-', text)
    return text.lower()

def get_all_product_links(url):
    """Finds all links on a page that appear to be product pages."""
    try:
        # First, find the "our-products" page from the base URL
        base_response = requests.get(url)
        base_response.raise_for_status()
        base_soup = BeautifulSoup(base_response.text, 'html.parser')
        
        our_products_link = None
        for a_tag in base_soup.find_all('a', href=True):
            if 'our-products' in a_tag['href']:
                our_products_link = urljoin(url, a_tag['href'])
                break
        
        if not our_products_link:
            print("Could not find the 'our-products' page link.")
            return []

        # Now, crawl the "our-products" page to find individual product links
        print(f"Found 'our-products' page: {our_products_link}")
        products_response = requests.get(our_products_link)
        products_response.raise_for_status()
        products_soup = BeautifulSoup(products_response.text, 'html.parser')

        links = set()
        for a_tag in products_soup.find_all('a', href=True):
            href = a_tag['href']
            # Make the URL absolute
            full_url = urljoin(url, href)
            # Check if it's a product URL
            if '/product-page/' in full_url or urlparse(full_url).path.startswith('/product/'):
                links.add(full_url)
        return list(links)
    except requests.exceptions.RequestException as e:
        print(f"Error fetching {url}: {e}")
        return []



def crawl_and_extract():
    """Main function to crawl the site, find product pages, and extract info."""
    print(f"Starting crawl of {BASE_URL} to find product pages...")
    product_links = get_all_product_links(BASE_URL)
    
    if not product_links:
        print("No product links found. Exiting.")
        return

    print(f"Found {len(product_links)} potential product pages.")
    
    for link in product_links:
        print(f"Processing: {link}")
        time.sleep(5) # Add a delay to avoid hitting rate limits
        try:
            response = requests.get(link)
            response.raise_for_status()
            
            soup = BeautifulSoup(response.text, 'html.parser')
            main_content = soup.find('main')
            if not main_content:
                print(f"Could not find main content for {link}. Skipping.")
                continue
            
            content_to_process = main_content.get_text(strip=True, separator='\n')

            # --- Step 1: Extract Product Name ---
            print("Extracting product name...")
            name_prompt = ChatPromptTemplate.from_messages([
                ("system", "You are an expert at extracting product names from text. Analyze the provided text and return only the product name."),
                ("user", "Page Content:\n{content}")
            ])
            name_chain = name_prompt | llm | StrOutputParser()
            product_name = name_chain.invoke({"content": content_to_process})

            if not product_name or "N/A" in product_name:
                print(f"Could not extract product name for URL: {link}. Skipping.")
                continue

            print(f"  - Found product name: {product_name}")

            # --- Step 2: Extract Remaining Details ---
            print("Extracting remaining details...")
            details_prompt = ChatPromptTemplate.from_messages([
                ("system", """You are an expert at extracting structured information from text.
                 Analyze the provided text from a product page and extract the following details.
                 Output the information in a clean, structured Markdown format, omitting the Product Name.
                 If a field is not present, use "N/A".
                 
                 The fields to extract are:
                 - Detailed Description
                 - Key Benefits (as a bulleted list)
                 - Ingredients
                 - Price and Sizes"""),
                ("user", "Page Content:\n{content}")
            ])
            details_chain = details_prompt | llm | StrOutputParser()
            product_details = details_chain.invoke({"content": content_to_process})

            # --- Step 3: Combine and Save ---
            filename = sanitize_filename(product_name) + ".md"
            filepath = os.path.join(PRODUCT_DIR, filename)
            
            full_product_info = f"Product Name: {product_name}\n\n{product_details}"
            
            print(f"Saving extracted info to {filepath}")
            with open(filepath, "w") as f:
                f.write(full_product_info)

        except requests.exceptions.RequestException as e:
            print(f"Error processing {link}: {e}")
        except Exception as e:
            print(f"An unexpected error occurred for {link}: {e}")

if __name__ == "__main__":
    crawl_and_extract()
