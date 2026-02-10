from typing import Dict, Any, List, Optional
from datetime import datetime, timedelta

class EventCalendarTool:
    def __init__(self):
        pass

    def is_available(self) -> bool:
        """
        (Conceptual mock tool) Always available.
        """
        return True

    def run(self, input_data: Dict[str, Any]) -> Dict[str, Any]:
        """
        Retrieves mock calendar events for a given date range, optionally filtered by keywords.
        Input: { "start_date": "YYYY-MM-DD", "end_date": "YYYY-MM-DD", "keywords": ["keyword1", "keyword2"] }
        Output: { "status": "success", "events": [...] } or { "status": "error", "message": "..." }
        """
        try:
            start_date_str = input_data.get("start_date")
            end_date_str = input_data.get("end_date")
            keywords: List[str] = [k.lower() for k in input_data.get("keywords", [])]

            if not start_date_str or not end_date_str:
                return {"status": "error", "message": "start_date and end_date are required."}

            start_date = datetime.fromisoformat(start_date_str)
            end_date = datetime.fromisoformat(end_date_str)

            mock_events = self._generate_mock_events()

            filtered_events = []
            for event in mock_events:
                event_date = datetime.fromisoformat(event["date"])
                if start_date <= event_date <= end_date:
                    if not keywords or any(k in event["description"].lower() for k in keywords):
                        filtered_events.append(event)
            
            return {"status": "success", "events": filtered_events}

        except Exception as e:
            return {"status": "error", "message": f"Error processing input: {e}"}

    def _generate_mock_events(self) -> List[Dict[str, Any]]:
        """
        Generates a list of mock calendar events.
        """
        today = datetime.now()
        return [
            {"id": "e001", "title": "Marketing Strategy Meeting", "date": (today - timedelta(days=2)).isoformat(), "description": "Review Q1 marketing strategy."}, 
            {"id": "e002", "title": "Product Launch Planning", "date": today.isoformat(), "description": "Finalize launch details for new dog food line."}, 
            {"id": "e003", "title": "Team Brainstorm: Social Media", "date": (today + timedelta(days=1)).isoformat(), "description": "Brainstorming session for upcoming social media campaign."}, 
            {"id": "e004", "title": "Content Calendar Review", "date": (today + timedelta(days=5)).isoformat(), "description": "Review blog posts and video content plan."}, 
            {"id": "e005", "title": "Client Meeting: PetCo", "date": (today + timedelta(days=10)).isoformat(), "description": "Discuss Q2 partnership with PetCo."}, 
            {"id": "e006", "title": "Budget Approval", "date": (today + timedelta(days=1)).isoformat(), "description": "Financial review and budget approval for next quarter."}, 
            {"id": "e007", "title": "SEO Workshop", "date": (today + timedelta(days=3)).isoformat(), "description": "Workshop on latest SEO techniques for pet niche."},
        ]

# Example Usage
if __name__ == "__main__":
    tool = EventCalendarTool()

    print("--- All events for next 7 days ---")
    result = tool.run({
        "start_date": datetime.now().isoformat().split('T')[0],
        "end_date": (datetime.now() + timedelta(days=7)).isoformat().split('T')[0]
    })
    print(json.dumps(result, indent=2))

    print("\n--- Events with 'planning' keyword ---")
    result_filtered = tool.run({
        "start_date": (datetime.now() - timedelta(days=10)).isoformat().split('T')[0],
        "end_date": (datetime.now() + timedelta(days=10)).isoformat().split('T')[0],
        "keywords": ["planning"]
    })
    print(json.dumps(result_filtered, indent=2))

    print("\n--- Invalid input ---")
    result_invalid = tool.run({})
    print(json.dumps(result_invalid, indent=2))
