import json
import os
from datetime import datetime, timedelta
from typing import List, Dict, Optional

class ScheduledTask:
    def __init__(self, task_id: str, frequency: str, last_run: Optional[datetime] = None, enabled: bool = True, data: Optional[Dict] = None):
        self.task_id = task_id
        self.frequency = frequency # e.g., "daily", "weekly", "monthly"
        self.last_run = last_run
        self.enabled = enabled
        self.data = data if data is not None else {}

    def to_dict(self):
        return {
            "task_id": self.task_id,
            "frequency": self.frequency,
            "last_run": self.last_run.isoformat() if self.last_run else None,
            "enabled": self.enabled,
            "data": self.data
        }

    @classmethod
    def from_dict(cls, data: Dict):
        last_run = datetime.fromisoformat(data["last_run"]) if data["last_run"] else None
        return cls(
            task_id=data["task_id"],
            frequency=data["frequency"],
            last_run=last_run,
            enabled=data.get("enabled", True),
            data=data.get("data", {})
        )

class ScheduledTaskRegistry:
    def __init__(self, storage_file: str = "scheduled_tasks.json"):
        self.storage_file = storage_file
        self.tasks: Dict[str, ScheduledTask] = self._load_tasks()

    def _load_tasks(self) -> Dict[str, ScheduledTask]:
        if os.path.exists(self.storage_file):
            with open(self.storage_file, 'r') as f:
                try:
                    raw_tasks = json.load(f)
                    return {t["task_id"]: ScheduledTask.from_dict(t) for t in raw_tasks}
                except json.JSONDecodeError:
                    return {}
        return {}

    def _save_tasks(self):
        with open(self.storage_file, 'w') as f:
            json.dump([task.to_dict() for task in self.tasks.values()], f, indent=4)

    def add_schedule(self, task: ScheduledTask):
        self.tasks[task.task_id] = task
        self._save_tasks()

    def get_schedule(self, task_id: str) -> Optional[ScheduledTask]:
        return self.tasks.get(task_id)

    def get_schedules(self, frequency: Optional[str] = None, enabled_only: bool = True) -> List[ScheduledTask]:
        filtered_tasks = []
        for task in self.tasks.values():
            if enabled_only and not task.enabled:
                continue
            if frequency and task.frequency != frequency:
                continue
            filtered_tasks.append(task)
        return filtered_tasks

    def update_last_run(self, task_id: str, last_run: datetime):
        task = self.tasks.get(task_id)
        if task:
            task.last_run = last_run
            self._save_tasks()

    def remove_schedule(self, task_id: str):
        if task_id in self.tasks:
            del self.tasks[task_id]
            self._save_tasks()

if __name__ == "__main__":
    registry = ScheduledTaskRegistry("test_scheduled_tasks.json")

    # Add a schedule
    task1 = ScheduledTask(task_id="daily_report", frequency="daily", data={"report_type": "summary"})
    registry.add_schedule(task1)
    print(f"Added task: {task1.task_id}")

    # Get schedules
    daily_tasks = registry.get_schedules(frequency="daily")
    print(f"Daily tasks: {[t.task_id for t in daily_tasks]}")

    # Update last run
    registry.update_last_run("daily_report", datetime.now())
    print(f"Updated last run for daily_report.")

    # Add another task, disabled
    task2 = ScheduledTask(task_id="weekly_newsletter", frequency="weekly", enabled=False)
    registry.add_schedule(task2)

    # Get enabled schedules
    all_enabled = registry.get_schedules(enabled_only=True)
    print(f"All enabled tasks: {[t.task_id for t in all_enabled]}")

    # Remove a schedule
    registry.remove_schedule("daily_report")
    print(f"Removed daily_report.")

    remaining_tasks = registry.get_schedules()
    print(f"Remaining tasks: {[t.task_id for t in remaining_tasks]}")

    # Clean up test file
    if os.path.exists("test_scheduled_tasks.json"):
        os.remove("test_scheduled_tasks.json")
