import os
from abc import ABC, abstractmethod
from typing import Dict, Any, Optional

# --- MarketingPlatformAdapter Interface ---
class MarketingPlatformAdapter(ABC):
    @abstractmethod
    def name(self) -> str:
        pass

    @abstractmethod
    def is_available(self) -> bool:
        """Checks if the adapter is configured and available (e.g., API keys present)."""
        pass

    @abstractmethod
    def deploy_campaign(self, campaign_data: Dict[str, Any]) -> Dict[str, Any]:
        """Deploys a campaign to the specific marketing platform."""
        pass

# --- Concrete Adapter Implementations (Mocks) ---
class FacebookAdapter(MarketingPlatformAdapter):
    def name(self) -> str:
        return "Facebook"

    def is_available(self) -> bool:
        return os.getenv("FACEBOOK_API_KEY") is not None

    def deploy_campaign(self, campaign_data: Dict[str, Any]) -> Dict[str, Any]:
        print(f"Deploying to Facebook with data: {campaign_data}")
        # Simulate API call
        if campaign_data.get("budget", 0) > 1000:
            return {"status": "success", "platform": "Facebook", "message": "Campaign deployed to Facebook (mock)."}
        else:
            return {"status": "error", "platform": "Facebook", "message": "Facebook campaign deployment failed (mock): budget too low."}

class GoogleAdsAdapter(MarketingPlatformAdapter):
    def name(self) -> str:
        return "GoogleAds"

    def is_available(self) -> bool:
        return os.getenv("GOOGLE_ADS_API_KEY") is not None

    def deploy_campaign(self, campaign_data: Dict[str, Any]) -> Dict[str, Any]:
        print(f"Deploying to Google Ads with data: {campaign_data}")
        # Simulate API call
        return {"status": "success", "platform": "GoogleAds", "message": "Campaign deployed to Google Ads (mock)."}

# --- MarketingPlatformManager Tool ---
class MarketingPlatformManager:
    def __init__(self):
        self._adapters: Dict[str, MarketingPlatformAdapter] = {}
        self._register_default_adapters()

    def _register_default_adapters(self):
        self.register_adapter(FacebookAdapter())
        self.register_adapter(GoogleAdsAdapter())

    def register_adapter(self, adapter: MarketingPlatformAdapter):
        self._adapters[adapter.name()] = adapter

    def is_available(self) -> bool:
        """Manager is available if at least one adapter is available."""
        return any(adapter.is_available() for adapter in self._adapters.values())

    def run(self, input_data: Dict[str, Any]) -> Dict[str, Any]:
        """
        Deploys a campaign to a specified marketing platform.
        Input: { "platform": "Facebook"|"GoogleAds", "campaign_data": { ... } }
        Output: { "status": "success", "platform": "...", "message": "..." } or { "status": "error", "message": "..." }
        """
        platform_name = input_data.get("platform")
        campaign_data = input_data.get("campaign_data", {})

        if not platform_name:
            return {"status": "error", "message": "Platform name is required."}

        adapter = self._adapters.get(platform_name)

        if not adapter:
            return {"status": "error", "message": f"Unknown marketing platform: {platform_name}"}

        if not adapter.is_available():
            return {"status": "error", "message": f"Marketing platform '{platform_name}' is not available (API keys missing?)."}

        return adapter.deploy_campaign(campaign_data)

# Example Usage
if __name__ == "__main__":
    manager = MarketingPlatformManager()

    # --- Test 1: Facebook - not available (no API key) ---
    print("\n--- Test 1: Facebook - not available ---")
    result1 = manager.run({
        "platform": "Facebook",
        "campaign_data": {"name": "Summer Sale", "budget": 500}
    })
    print(result1)

    # --- Test 2: Facebook - available (with API key) ---
    print("\n--- Test 2: Facebook - available and success ---")
    os.environ["FACEBOOK_API_KEY"] = "fake_fb_key"
    result2 = manager.run({
        "platform": "Facebook",
        "campaign_data": {"name": "Winter Campaign", "budget": 1500}
    })
    print(result2)

    print("\n--- Test 3: Facebook - available but deployment fails ---")
    result3 = manager.run({
        "platform": "Facebook",
        "campaign_data": {"name": "Low Budget Ad", "budget": 500}
    })
    print(result3)
    del os.environ["FACEBOOK_API_KEY"]


    # --- Test 4: GoogleAds - not available (no API key) ---
    print("\n--- Test 4: GoogleAds - not available ---")
    result4 = manager.run({
        "platform": "GoogleAds",
        "campaign_data": {"name": "Search Ads", "budget": 2000}
    })
    print(result4)

    # --- Test 5: GoogleAds - available (with API key) ---
    print("\n--- Test 5: GoogleAds - available and success ---")
    os.environ["GOOGLE_ADS_API_KEY"] = "fake_ga_key"
    result5 = manager.run({
        "platform": "GoogleAds",
        "campaign_data": {"name": "Display Ads", "budget": 3000}
    })
    print(result5)
    del os.environ["GOOGLE_ADS_API_KEY"]

    # --- Test 6: Unknown platform ---
    print("\n--- Test 6: Unknown platform ---")
    result6 = manager.run({
        "platform": "TikTok",
        "campaign_data": {"name": "TikTok Campaign", "budget": 100}
    })
    print(result6)
