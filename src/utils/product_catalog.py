import os
import re
from typing import List, Dict, Optional

class ProductCatalog:
    def __init__(self, products_dir: str):
        self.products_dir = products_dir
        self.product_names: List[str] = []
        self._load_products()

    def _load_products(self):
        """
        Loads product names from Markdown files in the specified directory.
        Assumes each product file has a "Product Name: <name>" line at the beginning.
        """
        if not os.path.exists(self.products_dir):
            print(f"Warning: Products directory not found at {self.products_dir}")
            return

        for filename in os.listdir(self.products_dir):
            if filename.endswith(".md"):
                filepath = os.path.join(self.products_dir, filename)
                with open(filepath, 'r', encoding='utf-8') as f:
                    first_line = f.readline().strip()
                    match = re.match(r"Product Name: (.*)", first_line)
                    if match:
                        product_name = match.group(1).strip()
                        self.product_names.append(product_name)
        print(f"Loaded {len(self.product_names)} product names from {self.products_dir}")

    def get_product_names(self) -> List[str]:
        """Returns a list of all loaded product names."""
        return self.product_names

    def product_exists(self, query_name: str) -> bool:
        """
        Checks if a product with the given name (case-insensitive) exists in the catalog.
        Also handles "Ready Raw® Beef (For Dogs)" vs "Ready Raw Beef (For Dogs)"
        """
        query_name_lower = query_name.lower().replace('®', '')
        for product_name in self.product_names:
            if query_name_lower == product_name.lower().replace('®', ''):
                return True
        return False

    def find_similar_product(self, query_name: str) -> Optional[str]:
        """
        Attempts to find a similar product name in the catalog based on the query,
        using a simple substring match (case-insensitive and ignoring ®).
        """
        query_name_lower = query_name.lower().replace('®', '')
        for product_name in self.product_names:
            product_name_lower = product_name.lower().replace('®', '')
            if query_name_lower in product_name_lower or product_name_lower in query_name_lower:
                return product_name
        return None

if __name__ == "__main__":
    # Example usage:
    # Assuming the script is run from the project root
    PRODUCTS_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data", "products")
    
    catalog = ProductCatalog(PRODUCTS_DIR)
    
    print("\nAll Products:")
    for name in catalog.get_product_names():
        print(f"- {name}")

    print("\nChecking product existence:")
    print(f"'Ready Raw® Beef (For Dogs)' exists: {catalog.product_exists('Ready Raw® Beef (For Dogs)')}")
    print(f"'ready raw chicken for cats' exists: {catalog.product_exists('ready raw chicken for cats')}")
    print(f"'Whiskers & Wellness Organic Salmon Feast' exists: {catalog.product_exists('Whiskers & Wellness Organic Salmon Feast')}")
    print(f"'Non-existent product' exists: {catalog.product_exists('Non-existent product')}")

    print("\nFinding similar products:")
    print(f"Similar to 'Raw Beef': {catalog.find_similar_product('Raw Beef')}")
    print(f"Similar to 'Chicken for Cats': {catalog.find_similar_product('Chicken for Cats')}")
    print(f"Similar to 'Dental Powder': {catalog.find_similar_product('Dental Powder')}")
