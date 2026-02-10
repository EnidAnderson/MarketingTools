import subprocess
import os
from typing import Dict, Any, List

class GifGeneratorTool:
    def __init__(self):
        pass

    def _run_ffmpeg_command(self, args: List[str]) -> bool:
        """Helper to run an ffmpeg command and return success status."""
        try:
            command = ["ffmpeg"] + args
            result = subprocess.run(command, capture_output=True, text=True, check=True)
            print(f"FFmpeg command successful: {' '.join(command)}")
            # print(f"Stdout: {result.stdout}")
            return True
        except subprocess.CalledProcessError as e:
            print(f"FFmpeg command failed: {' '.join(command)}")
            print(f"Stderr: {e.stderr}")
            return False
        except FileNotFoundError:
            print("'ffmpeg' command not found. Please ensure FFmpeg is installed and in your PATH.")
            return False

    def is_available(self) -> bool:
        """
        Checks if the 'ffmpeg' command is available in the system's PATH.
        """
        return self._run_ffmpeg_command(["-version"])

    def run(self, input_data: Dict[str, Any]) -> Dict[str, Any]:
        """
        Generates a GIF from a list of input image paths.
        Input: { 
            "input_images": ["path/to/img1.png", "path/to/img2.png"],
            "output_path": "path/to/output.gif",
            "frame_rate": 10,  # frames per second
            "loop_count": 0    # 0 for infinite loop, -1 for no loop, or a positive integer
        }
        Output: { "status": "success", "message": "..." } or { "status": "error", "message": "..." }
        """
        input_images: List[str] = input_data.get("input_images", [])
        output_path = input_data.get("output_path")
        frame_rate = input_data.get("frame_rate", 10)
        loop_count = input_data.get("loop_count", 0)

        if not input_images:
            return {"status": "error", "message": "input_images list is required and cannot be empty."}
        if not output_path:
            return {"status": "error", "message": "output_path is required."}
        
        for img_path in input_images:
            if not os.path.exists(img_path):
                return {"status": "error", "message": f"Input image not found: {img_path}"}

        # FFmpeg command for GIF creation:
        # ffmpeg -i img1.png -i img2.png -filter_complex "[0:v][1:v]...[n:v]concat=n=N:v=1:a=0[v]" -map "[v]" -r 10 -loop 0 output.gif
        # A simpler approach for sequential images:
        # ffmpeg -f image2 -i input_%d.png -r 10 -loop 0 output.gif
        # For an arbitrary list of images, we can use a concat demuxer or pipe.
        # For simplicity in this mock, we'll simulate the arguments:
        # ffmpeg -i img1.png -i img2.png ... -r <frame_rate> -loop <loop_count> output.gif
        
        args: List[str] = []
        for img_path in input_images:
            args.extend(["-i", img_path])
        
        args.extend(["-vf", "fps={}".format(frame_rate)]) # Set frame rate
        args.extend(["-loop", str(loop_count)]) # Set loop count
        args.append(output_path)

        if self._run_ffmpeg_command(args):
            return {"status": "success", "message": f"GIF generated successfully. Output: {output_path}"}
        else:
            return {"status": "error", "message": "GIF generation failed."}

# Example Usage (requires FFmpeg installed and dummy files)
if __name__ == "__main__":
    tool = GifGeneratorTool()
    
    # Create dummy files for testing
    dummy_img1_path = "dummy_img1.png"
    dummy_img2_path = "dummy_img2.png"
    with open(dummy_img1_path, "w") as f: f.write("dummy_png_content_1")
    with open(dummy_img2_path, "w") as f: f.write("dummy_png_content_2")

    if tool.is_available():
        print("FFmpeg 'ffmpeg' command is available.")

        # Test GIF generation
        print("\n--- Testing GIF generation ---")
        gif_result = tool.run({
            "input_images": [dummy_img1_path, dummy_img2_path],
            "output_path": "output.gif",
            "frame_rate": 5,
            "loop_count": 0
        })
        print(gif_result)

        # Test missing input images
        print("\n--- Testing missing input images ---")
        missing_images_result = tool.run({
            "input_images": ["non_existent.png"],
            "output_path": "output_fail.gif"
        })
        print(missing_images_result)

    else:
        print("FFmpeg 'ffmpeg' command is NOT available.")

    # Clean up dummy files
    if os.path.exists(dummy_img1_path): os.remove(dummy_img1_path)
    if os.path.exists(dummy_img2_path): os.remove(dummy_img2_path)
    if os.path.exists("output.gif"): os.remove("output.gif")
    if os.path.exists("output_fail.gif"): os.remove("output_fail.gif")
