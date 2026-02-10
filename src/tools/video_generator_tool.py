import os
import subprocess
from typing import Optional
from src.config import PROJECT_ROOT # Assuming PROJECT_ROOT is defined in src/config.py

class VideoGeneratorTool:
    """
    A tool to generate video content based on provided specifications.
    Currently simulates video generation.
    """
    def __init__(self):
        pass

    def is_available(self) -> bool:
        """
        Checks if ffmpeg is installed and available in the system's PATH.
        """
        try:
            subprocess.run(["ffmpeg", "-version"], capture_output=True, check=True)
            return True
        except (subprocess.CalledProcessError, FileNotFoundError):
            return False

    def run(
        self,
        video_script: str,
        video_style: str,
        video_duration_seconds: int,
        video_assets_description: str,
        campaign_dir: str
    ) -> str:
        """
        Simulates generating a video based on the provided specifications.
        In a real scenario, this would orchestrate ffmpeg commands or external video APIs.

        Returns:
            str: Path to the generated video file, or an error message.
        """
        if not self.is_available():
            return "Error: ffmpeg is not installed or not in PATH. Cannot generate video."

        print("--- Simulating Video Generation ---")
        print(f"Script: {video_script[:100]}...")
        print(f"Style: {video_style}")
        print(f"Duration: {video_duration_seconds}s")
        print(f"Assets: {video_assets_description[:100]}...")

        output_video_filename = f"generated_video_{video_duration_seconds}s_{video_style.replace(' ', '_')}.mp4"
        output_video_path = os.path.join(campaign_dir, output_video_filename)
        os.makedirs(campaign_dir, exist_ok=True)

        # Simulate creating a very basic blank video with ffmpeg
        try:
            # Create a 1-second blank video with some text overlay to show it's "generated"
            # For a real video, this would involve much more complex ffmpeg commands
            # using multiple inputs, filters, etc.
            temp_text_image = os.path.join(campaign_dir, "temp_text.png")
            
            # Use imagemagick to create an image with text (simulating video content)
            # This requires imagemagick 'magick' command. If not available, skip text.
            try:
                subprocess.run([
                    "magick", "convert", "-size", "1280x720", "xc:white",
                    "-font", "Arial", "-pointsize", "48", "-fill", "black",
                    "-gravity", "Center", "-annotate", "+0+0",
                    f"Simulated Video: {video_style}\nDuration: {video_duration_seconds}s",
                    temp_text_image
                ], capture_output=True, check=True)
                has_imagemagick = True
            except (subprocess.CalledProcessError, FileNotFoundError):
                print("Warning: ImageMagick 'magick' command not found. Generating blank video.")
                has_imagemagick = False

            if has_imagemagick:
                ffmpeg_cmd = [
                    "ffmpeg", "-y", "-loop", "1", "-i", temp_text_image,
                    "-c:v", "libx264", "-t", str(video_duration_seconds),
                    "-pix_fmt", "yuv420p", "-vf", "scale=trunc(iw/2)*2:trunc(ih/2)*2",
                    output_video_path
                ]
            else:
                ffmpeg_cmd = [
                    "ffmpeg", "-y",
                    "-f", "lavfi", "-i", "color=c=black:s=1280x720:d=1",
                    "-vf", f"drawtext=fontfile=Arial:text='Simulated Video':x=(w-text_w)/2:y=(h-text_h)/2:fontsize=48:fontcolor=white",
                    "-t", str(video_duration_seconds),
                    output_video_path
                ]

            subprocess.run(ffmpeg_cmd, capture_output=True, check=True)
            
            if has_imagemagick:
                os.remove(temp_text_image) # Clean up temp image

            print(f"Simulated video saved to {output_video_path}")
            return output_video_path
        except subprocess.CalledProcessError as e:
            error_output = e.stderr.decode()
            print(f"Error during simulated video generation: {e.cmd}\n{error_output}")
            return f"Error generating video: {error_output}"
        except Exception as e:
            print(f"An unexpected error occurred during video generation: {e}")
            return f"An unexpected error occurred during video generation: {e}"

if __name__ == "__main__":
    # Example Usage
    tool = VideoGeneratorTool()
    if tool.is_available():
        print("ffmpeg is available. Running simulation.")
        dummy_campaign_dir = os.path.join(PROJECT_ROOT, "CAMPAIGNS", "dummy_video_campaign")
        os.makedirs(dummy_campaign_dir, exist_ok=True)
        
        video_path = tool.run(
            video_script="Show happy dog eating, then product shots, then call to action.",
            video_style="product showcase",
            video_duration_seconds=5,
            video_assets_description="Multiple product shots, happy dog stock footage.",
            campaign_dir=dummy_campaign_dir
        )
        print(f"Video simulation result: {video_path}")
    else:
        print("ffmpeg is not available. Cannot run video generation simulation.")
