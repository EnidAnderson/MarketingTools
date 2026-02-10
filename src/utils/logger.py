import logging
import json
import threading

_current_campaign_id = threading.local()
_current_campaign_id.value = None

class JsonFormatter(logging.Formatter):
    def format(self, record):
        log_record = {
            "timestamp": self.formatTime(record, self.datefmt),
            "level": record.levelname,
            "message": record.getMessage(),
            "campaign_id": getattr(_current_campaign_id, "value", None),
            "name": record.name,
            "pathname": record.pathname,
            "lineno": record.lineno,
            "funcName": record.funcName,
            "process": record.process,
            "thread": record.thread,
        }
        if hasattr(record, 'extra_data'):
            log_record.update(record.extra_data)
        return json.dumps(log_record)

def set_current_campaign_id(campaign_id: str):
    _current_campaign_id.value = campaign_id

def log_agent_event(agent_name: str, event_type: str, details: dict):
    logger = logging.getLogger(agent_name)
    logger.info("Agent Event", extra={"extra_data": {"event_type": event_type, "details": details}})

def log_llm_call(agent_name: str, prompt: str, response: str, model: str):
    logger = logging.getLogger(agent_name)
    logger.info("LLM Call", extra={"extra_data": {"prompt": prompt, "response": response, "model": model}})

def log_tool_use(agent_name: str, tool_name: str, input_data: dict, output_data: dict):
    logger = logging.getLogger(agent_name)
    logger.info("Tool Use", extra={"extra_data": {"tool_name": tool_name, "input": input_data, "output": output_data}})

# Basic setup for demonstration
if __name__ == "__main__":
    handler = logging.StreamHandler()
    formatter = JsonFormatter()
    handler.setFormatter(formatter)

    root_logger = logging.getLogger()
    root_logger.setLevel(logging.INFO)
    root_logger.addHandler(handler)

    set_current_campaign_id("test_campaign_123")
    logging.info("This is a root logger message.")

    my_agent_logger = logging.getLogger("StrategistAgent")
    my_agent_logger.info("Strategist agent started.")

    log_agent_event("StrategistAgent", "task_started", {"task": "initial_planning"})
    log_llm_call("StrategistAgent", "Plan a campaign", "Campaign plan generated.", "gemini-pro")
    log_tool_use("StrategistAgent", "EmailSenderTool", {"to": "test@example.com"}, {"status": "success"})

    set_current_campaign_id("another_campaign_456")
    logging.warning("This is a warning for another campaign.")
