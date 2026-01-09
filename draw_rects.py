import sys
import re
import argparse
import math
from PIL import Image, ImageDraw, ImageFont, ImageStat

# === PARSING LOGIC ===
def parse_capstone_data(input_text):
    """
    Parses logs strictly.
    Only captures data specifically inside a '--- Capstone #X ---' block.
    """
    capstones = []
    in_capstone_block = False
    current_capstone = None

    capstone_header_pattern = re.compile(r"--- Capstone #(\d+) ---")
    section_separator_pattern = re.compile(r"========== .* ==========")

    center_pattern = re.compile(r"(?<!\w)Center:\s*\((\d+),\s*(\d+)\)")
    # Group 2 is X, Group 3 is Y
    corner_pattern = re.compile(r"Corner\s*(\d+):\s*\((\d+),\s*(\d+)\)")

    lines = input_text.splitlines()

    def save_current():
        nonlocal current_capstone
        if current_capstone and (current_capstone['center'] or current_capstone['corners']):
            capstones.append(current_capstone)
        current_capstone = None

    for line in lines:
        if section_separator_pattern.search(line):
            save_current()
            in_capstone_block = False
            continue

        header_match = capstone_header_pattern.search(line)
        if header_match:
            save_current()
            in_capstone_block = True
            c_id = int(header_match.group(1))
            current_capstone = {'id': c_id, 'center': None, 'corners': []}
            continue

        if not in_capstone_block or current_capstone is None:
            continue

        center_match = center_pattern.search(line)
        if center_match:
            x, y = int(center_match.group(1)), int(center_match.group(2))
            current_capstone['center'] = (x, y)
            continue

        corner_match = corner_pattern.search(line)
        if corner_match:
            x, y = int(corner_match.group(2)), int(corner_match.group(3))
            current_capstone['corners'].append((x, y))

    save_current()
    return capstones

# === COLOR LOGIC ===
def get_adaptive_color(image, x, y):
    """Returns Green, or Magenta if background is too close to Green."""
    w, h = image.size
    x1, y1 = max(0, int(x) - 2), max(0, int(y) - 2)
    x2, y2 = min(w, int(x) + 3), min(h, int(y) + 3)

    crop = image.crop((x1, y1, x2, y2))
    stat = ImageStat.Stat(crop)

    bg = stat.mean[:3]
    if len(bg) < 3: bg = (bg[0], bg[0], bg[0])

    dist_green = math.sqrt((bg[0]-0)**2 + (bg[1]-255)**2 + (bg[2]-0)**2)
    return (255, 0, 255) if dist_green < 100 else (0, 255, 0)

# === COLLISION DETECTION ===
def rects_intersect(r1, r2):
    """
    Returns True if two rectangles intersect.
    Rect format: (min_x, min_y, max_x, max_y)
    """
    return not (r1[2] < r2[0] or r1[0] > r2[2] or r1[3] < r2[1] or r1[1] > r2[3])

def is_position_safe(candidate_rect, forbidden_zones, image_size):
    """
    Checks if candidate_rect overlaps with any forbidden zone.
    Also checks if it fits within image boundaries.
    """
    cx0, cy0, cx1, cy1 = candidate_rect
    w, h = image_size

    # Check boundaries
    if cx0 < 0 or cy0 < 0 or cx1 > w or cy1 > h:
        return False

    # Check collision with other objects
    for zone in forbidden_zones:
        if rects_intersect(candidate_rect, zone):
            return False
    return True

def draw_label_smart(draw_ctx, cap_id, corners, font, forbidden_zones, image_size):
    label = f"#{cap_id}"

    # Text dimensions
    bbox = draw_ctx.textbbox((0, 0), label, font=font)
    text_w = bbox[2] - bbox[0]
    text_h = bbox[3] - bbox[1]

    # Calculate Capstone Bounding Box
    xs = [p[0] for p in corners]
    ys = [p[1] for p in corners]
    c_min_x, c_min_y = min(xs), min(ys)
    c_max_x, c_max_y = max(xs), max(ys)

    margin = 5

    # Priority Candidates:
    # 1. Top-Left (Outside)
    # 2. Top-Right (Outside)
    # 3. Bottom-Left (Outside)
    # 4. Bottom-Right (Outside)
    # 5. Top-Left (Inside - Last resort)

    candidates = [
        (c_min_x, c_min_y - text_h - margin),           # Above-Left
        (c_max_x - text_w, c_min_y - text_h - margin),  # Above-Right
        (c_min_x, c_max_y + margin),                    # Below-Left
        (c_max_x - text_w, c_max_y + margin)            # Below-Right
    ]

    chosen_rect = None

    for x, y in candidates:
        # Create rect for this candidate
        cand_rect = (x, y, x + text_w, y + text_h)

        # Check against ALL forbidden zones (other capstones + other texts)
        if is_position_safe(cand_rect, forbidden_zones, image_size):
            chosen_rect = cand_rect
            break

    # Fallback: If all outside corners are blocked, just force it Top-Left
    # (Better to draw overlapping than nothing)
    if not chosen_rect:
        x, y = candidates[0]
        chosen_rect = (x, y, x + text_w, y + text_h)

    # Draw
    draw_ctx.text((chosen_rect[0], chosen_rect[1]), label, fill="red", font=font)

    # Add new text area to forbidden zones so future texts don't overlap it
    forbidden_zones.append(chosen_rect)

# === MAIN PROCESS ===
def process_image(input_path, output_path, capstones):
    try:
        with Image.open(input_path) as img:
            img = img.convert("RGB")
            scale = 2
            new_size = (img.width * scale, img.height * scale)
            scaled_img = img.resize(new_size, resample=Image.NEAREST)

            draw = ImageDraw.Draw(scaled_img)

            try:
                font = ImageFont.truetype("arial.ttf", 18)
            except IOError:
                font = ImageFont.load_default()

            print(f"Processing {len(capstones)} capstones...")

            # 1. Build Forbidden Zones (Bounding Boxes of all capstones)
            # This ensures we treat the INSIDE of every capstone as 'occupied'
            forbidden_zones = []

            for cap in capstones:
                if cap['corners']:
                    pts = [(x * scale, y * scale) for x, y in cap['corners']]
                    xs = [p[0] for p in pts]
                    ys = [p[1] for p in pts]
                    # Store (min_x, min_y, max_x, max_y)
                    # Add a tiny buffer (2px) to make sure text doesn't touch lines
                    bbox = (min(xs)-2, min(ys)-2, max(xs)+2, max(ys)+2)
                    forbidden_zones.append(bbox)

            # 2. Draw Geometry (Rectangles & Dots)
            for cap in capstones:
                if cap['corners']:
                    pts = [(x * scale, y * scale) for x, y in cap['corners']]
                    draw.polygon(pts, outline=(255, 0, 0), width=1)

                if cap['center']:
                    cx, cy = cap['center']
                    sx, sy = cx * scale, cy * scale
                    color = get_adaptive_color(scaled_img, sx, sy)
                    draw.ellipse((sx-1, sy-1, sx+1, sy+1), fill=color, outline=None)

            # 3. Draw Labels (Smart Placement)
            for cap in capstones:
                if cap['corners']:
                    scaled_corners = [(x * scale, y * scale) for x, y in cap['corners']]
                    draw_label_smart(draw, cap['id'], scaled_corners, font, forbidden_zones, new_size)

            scaled_img.save(output_path)
            print(f"Saved: {output_path}")

    except Exception as e:
        print(f"Error processing image: {e}")
        sys.exit(1)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    if sys.stdin.isatty():
        print("Waiting for log data on stdin...")

    log_data = sys.stdin.read()
    if not log_data:
        print("Error: No data on stdin.")
        sys.exit(1)

    capstones = parse_capstone_data(log_data)

    if not capstones:
        print("Warning: No capstones found.")

    process_image(args.input, args.output, capstones)

if __name__ == "__main__":
    main()
