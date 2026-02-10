import os
import smtplib
from email.mime.text import MIMEText
from typing import Dict, Any

class EmailSenderTool:
    def __init__(self):
        # Configuration details for SMTP server (typically from environment variables)
        self.smtp_host = os.getenv("SMTP_HOST")
        self.smtp_port = int(os.getenv("SMTP_PORT", 587))
        self.smtp_user = os.getenv("SMTP_USERNAME")
        self.smtp_password = os.getenv("SMTP_PASSWORD")

    def is_available(self) -> bool:
        """
        Checks if all necessary SMTP environment variables are set.
        """
        return all([self.smtp_host, self.smtp_user, self.smtp_password])

    def run(self, input_data: Dict[str, Any]) -> Dict[str, Any]:
        """
        Sends an email.
        Input: { "to": "recipient@example.com", "subject": "Hello", "body": "Email body." }
        Output: { "status": "success", "message": "Email sent successfully." } or { "status": "error", "message": "..." }
        """
        if not self.is_available():
            return {"status": "error", "message": "SMTP configuration not complete."}

        to_email = input_data.get("to")
        subject = input_data.get("subject", "No Subject")
        body = input_data.get("body", "")

        if not to_email:
            return {"status": "error", "message": "Recipient 'to' email is required."}

        msg = MIMEText(body)
        msg["Subject"] = subject
        msg["From"] = self.smtp_user
        msg["To"] = to_email

        try:
            with smtplib.SMTP(self.smtp_host, self.smtp_port) as server:
                server.starttls()  # Secure the connection
                server.login(self.smtp_user, self.smtp_password)
                server.send_message(msg)
            return {"status": "success", "message": "Email sent successfully."}
        except Exception as e:
            return {"status": "error", "message": f"Failed to send email: {e}"}

# Example Usage
if __name__ == "__main__":
    # For demonstration, set dummy environment variables
    os.environ["SMTP_HOST"] = "smtp.example.com"
    os.environ["SMTP_PORT"] = "587"
    os.environ["SMTP_USERNAME"] = "your_email@example.com"
    os.environ["SMTP_PASSWORD"] = "your_password"

    tool = EmailSenderTool()

    if tool.is_available():
        print("EmailSenderTool is available.")
        # Example success
        result = tool.run({
            "to": "test@example.com",
            "subject": "Test Email from Python",
            "body": "This is a test email sent using the EmailSenderTool."
        })
        print(f"Result 1: {result}")

        # Example failure (missing 'to')
        result_fail = tool.run({
            "subject": "Failing Test",
            "body": "This email should fail."
        })
        print(f"Result 2: {result_fail}")
    else:
        print("EmailSenderTool is not available due to missing SMTP config.")

    # Clean up dummy environment variables
    del os.environ["SMTP_HOST"]
    del os.environ["SMTP_PORT"]
    del os.environ["SMTP_USERNAME"]
    del os.environ["SMTP_PASSWORD"]
