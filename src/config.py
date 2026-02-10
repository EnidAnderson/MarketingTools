import os

# Get the absolute path of the project's root directory
PROJECT_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# --- Path Configurations ---
DATA_PATH = os.path.join(PROJECT_ROOT, "src", "data")
PRODUCT_DIR = os.path.join(DATA_PATH, "products")
PERSIST_DIRECTORY = os.path.join(PROJECT_ROOT, "src", "chroma_db")
BUDGET_FILE = os.path.join(DATA_PATH, "generation_budget.json")
SCREENSHOTS_DIR = os.path.join(PROJECT_ROOT, "screenshots") # New path

# --- URLs and Naming ---

# Official URLs for Nature's Diet Pet
TRUSTED_URLS = [
    "https://naturesdietpet.com",
    "https://naturesdietpet.com/blog",
    "https://www.amazon.com/stores/NaturesDiet%C2%AE/page/99AF1240-69C2-49A8-8888-6353796DC2AC?lp_asin=B08HJSMPWC&ref_=ast_bln&store_ref=bl_ast_dp_brandLogo_sto"
]

# Naming Conventions (Style Guide)
COMPANY_NAME = "Nature's Diet®"
FLAGSHIP_PRODUCT = "Simply Raw®"



# Budget Configuration for Paid Generative Services
BUDGET_CONFIG = {
    "daily_budget_usd": 2.00,  # Max USD spend per day
    "monthly_limit": 100,  # Max number of paid generations per month
    "time_window_hours": 5, # The time window in hours
    "time_window_limit": 25, # Max number of paid generations in the time window
}

# Refinement Loop Configuration
MAX_ITERATIONS = 1 # Maximum number of refinement iterations