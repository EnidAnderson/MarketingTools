import os
from dotenv import load_dotenv
from langchain_community.document_loaders import WebBaseLoader, UnstructuredMarkdownLoader
from langchain.text_splitter import RecursiveCharacterTextSplitter
from langchain_google_genai import GoogleGenerativeAIEmbeddings
from langchain_community.vectorstores import Chroma
from src.config import TRUSTED_URLS, DATA_PATH, PERSIST_DIRECTORY, SCREENSHOTS_DIR, PROJECT_ROOT
from src.tools.product_crawler import crawl_and_extract
from src.agents.design_analyst import DesignAnalystAgent # Added import
from src.utils.product_catalog import ProductCatalog # NEW IMPORT

load_dotenv()

# Ensure GOOGLE_API_KEY is set as an environment variable
if not os.getenv("GOOGLE_API_KEY"):
    raise ValueError("GOOGLE_API_KEY environment variable not set.")

llm_embeddings = GoogleGenerativeAIEmbeddings(model="models/embedding-001")

def build_vectorstore():
    """
    Builds or updates the ChromaDB vector store.
    1. Crawls the website for verified product information.
    2. Analyzes the website design and creates a design system document.
    3. Loads local markdown files (products, brand guidelines, image prompting guide, design system).
    4. Loads content from trusted URLs.
    5. Creates and persists the vector store.
    """
    print("--- Step 1: Crawling for verified product information ---")
    crawl_and_extract()
    print("--- Product crawl complete ---")

    print("\n--- Step 2: Analyzing website design and creating design system ---")
    design_analyst_agent = DesignAnalystAgent()
    design_system_template_path = os.path.join(DATA_PATH, "design_system", "design_system_template.md")
    current_design_system_path = os.path.join(DATA_PATH, "design_system", "current_design_system.md")
    
    # Ensure the design_system directory exists
    os.makedirs(os.path.dirname(current_design_system_path), exist_ok=True)

    filled_design_system_doc = design_analyst_agent.analyze_design(
        url="https://www.naturesdietpet.com",
        template_path=design_system_template_path
    )
    with open(current_design_system_path, "w") as f:
        f.write(filled_design_system_doc)
    print(f"Filled Design System saved to: {current_design_system_path}")
    print("--- Design analysis complete ---")


    print("\n--- Step 3: Building/updating vector store ---")

    documents = []

    # Initialize ProductCatalog
    products_dir = os.path.join(PROJECT_ROOT, "src", "data", "products")
    product_catalog = ProductCatalog(products_dir)
    
    # Load specific markdown files
    md_files_to_load = [
        os.path.join(PROJECT_ROOT, "src", "data", "brand_guidelines.md"),
        os.path.join(PROJECT_ROOT, "src", "data", "IMAGE_PROMPTING_GUIDE.md"),
        current_design_system_path, # Path to the generated design system
    ]

    # Load all product markdown files
    print(f"Loading product markdown documents from {products_dir}...")
    for product_filename in os.listdir(products_dir):
        if product_filename.endswith(".md"):
            file_path = os.path.join(products_dir, product_filename)
            loader = UnstructuredMarkdownLoader(file_path)
            documents.extend(loader.load())

    print(f"Loading other specific markdown documents...")
    for file_path in md_files_to_load:
        if os.path.exists(file_path):
            loader = UnstructuredMarkdownLoader(file_path)
            documents.extend(loader.load())
        else:
            print(f"Warning: Markdown file not found: {file_path}")

    print(f"Loaded {len(documents)} local markdown documents.")

    # Load web content from trusted URLs
    print(f"Loading content from trusted URLs: {TRUSTED_URLS}...")
    web_loader = WebBaseLoader(TRUSTED_URLS)
    web_documents = web_loader.load()
    documents.extend(web_documents)
    print(f"Loaded {len(web_documents)} web documents. Total documents: {len(documents)}")

    # Split documents
    print("Splitting documents into chunks...")
    text_splitter = RecursiveCharacterTextSplitter(chunk_size=1000, chunk_overlap=200)
    splits = text_splitter.split_documents(documents)
    print(f"Split into {len(splits)} chunks.")

    # Create and persist vector store
    print("Creating/updating ChromaDB vector store...")
    vectorstore = Chroma.from_documents(
        documents=splits,
        embedding=llm_embeddings,
        persist_directory=PERSIST_DIRECTORY
    )
    vectorstore.persist()
    print(f"Vector store built/updated successfully at {PERSIST_DIRECTORY}")

if __name__ == "__main__":
    build_vectorstore()