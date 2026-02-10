import subprocess
import os
from typing import Dict, Any, List

class ImageManipulationTool:
    def __init__(self):
        pass

    def _run_magick_command(self, args: List[str]) -> bool:
        """Helper to run a magick command and return success status."""
        try:
            # Use 'magick' command (ImageMagick v7+)
            command = ["magick"] + args
            result = subprocess.run(command, capture_output=True, text=True, check=True)
            print(f"Magick command successful: {' '.join(command)}")
            # print(f"Stdout: {result.stdout}")
            return True
        except subprocess.CalledProcessError as e:
            print(f"Magick command failed: {' '.join(command)}")
            print(f"Stderr: {e.stderr}")
            return False
        except FileNotFoundError:
            print("'magick' command not found. Please ensure ImageMagick is installed and in your PATH.")
            return False

    def is_available(self) -> bool:
        """
        Checks if the 'magick' command is available in the system's PATH.
        """
        return self._run_magick_command(["-version"])

    def run(self, input_data: Dict[str, Any]) -> Dict[str, Any]:
        """
        Performs image manipulation (resize or add_watermark).
        Input: { 
            "action": "resize" | "add_watermark",
            "input_path": "path/to/input.jpg",
            "output_path": "path/to/output.jpg",
            "width": 800 (for resize),
            "height": 600 (for resize),
            "watermark_path": "path/to/watermark.png" (for add_watermark),
            "gravity": "SouthEast" (for add_watermark, e.g., NorthWest, SouthEast)
        }
        Output: { "status": "success", "message": "..." } or { "status": "error", "message": "..." }
        """
        action = input_data.get("action")
        input_path = input_data.get("input_path")
        output_path = input_data.get("output_path")

        if not input_path or not output_path:
            return {"status": "error", "message": "input_path and output_path are required."}
        if not os.path.exists(input_path):
             return {"status": "error", "message": f"Input file not found: {input_path}"}

        args: List[str] = [input_path]

        if action == "resize":
            width = input_data.get("width")
            height = input_data.get("height")
            if not width and not height:
                return {"status": "error", "message": "For 'resize' action, 'width' or 'height' is required."}
            
            size_arg = ""
            if width and height:
                size_arg = f"{width}x{height}"
            elif width:
                size_arg = f"{width}x"
            elif height:
                size_arg = f"x{height}"
            
            args.extend(["-resize", size_arg, output_path])
        
        elif action == "add_watermark":
            watermark_path = input_data.get("watermark_path")
            gravity = input_data.get("gravity", "SouthEast") # Default gravity
            
            if not watermark_path:
                return {"status": "error", "message": "For 'add_watermark' action, 'watermark_path' is required."}
            if not os.path.exists(watermark_path):
                return {"status": "error", "message": f"Watermark file not found: {watermark_path}"}
            
            # Magick command for watermarking: magick input.jpg watermark.png -gravity SouthEast -composite output.jpg
            args = [input_path, watermark_path, "-gravity", gravity, "-composite", output_path]

        else:
            return {"status": "error", "message": "Invalid action. Must be 'resize' or 'add_watermark'."}

        if self._run_magick_command(args):
            return {"status": "success", "message": f"Image manipulation '{action}' successful. Output: {output_path}"}
        else:
            return {"status": "error", "message": f"Image manipulation '{action}' failed."}

# Example Usage (requires ImageMagick installed and dummy files)
if __name__ == "__main__":
    tool = ImageManipulationTool()
    
    # Create dummy files for testing
    dummy_input_path = "dummy_input.jpg"
    dummy_watermark_path = "dummy_watermark.png"
    with open(dummy_input_path, "w") as f: f.write("dummy_jpeg_content")
    with open(dummy_watermark_path, "w") as f: f.write("dummy_png_content")


    if tool.is_available():
        print("ImageMagick 'magick' command is available.")

        # Test resize
        print("\n--- Testing resize ---")
        resize_result = tool.run({
            "action": "resize",
            "input_path": dummy_input_path,
            "output_path": "resized_output.jpg",
            "width": 100,
            "height": 100
        })
        print(resize_result)

        # Test add_watermark
        print("\n--- Testing add_watermark ---")
        watermark_result = tool.run({
            "action": "add_watermark",
            "input_path": dummy_input_path,
            "output_path": "watermarked_output.jpg",
            "watermark_path": dummy_watermark_path,
            "gravity": "NorthWest"
        })
        print(watermark_result)

        # Test invalid action
        print("\n--- Testing invalid action ---")
        invalid_action_result = tool.run({
            "action": "crop",
            "input_path": dummy_input_path,
            "output_path": "cropped_output.jpg"
        })
        print(invalid_action_result)

    else:
        print("ImageMagick 'magick' command is NOT available.")

    # Clean up dummy files
    if os.path.exists(dummy_input_path): os.remove(dummy_input_path)
    if os.path.exists(dummy_watermark_path): os.remove(dummy_watermark_path)
    if os.path.exists("resized_output.jpg"): os.remove("resized_output.jpg")
    if os.path.exists("watermarked_output.jpg"): os.remove("watermarked_output.jpg")
