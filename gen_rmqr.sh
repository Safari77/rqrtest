#!/bin/bash

# Create an output directory
OUTPUT_DIR="rmqr_versions"
mkdir -p "$OUTPUT_DIR"

echo "Starting rMQR Brute-force Generator (Versions 1-31)..."

for n in {1..32}; do
    # 1. Generate initial random string
    RNDSTRING=$(randword 1 160 | tr -d '\n\r')

    echo -n "Processing Version $n... "

    while true; do
        DATA="VER$n $RNDSTRING"

        # Define output filenames
        IMG_FILE="${OUTPUT_DIR}/rmqr_v${n}.png"
        TXT_FILE="${OUTPUT_DIR}/rmqr_v${n}.txt"

        rm -f "$IMG_FILE"
        # Run zint
        zint --quietzone --scale 10 --whitesp=1 --vwhitesp=1 \
             -b rmqr --vers="$n" \
              \
             -d "$DATA" -o "$IMG_FILE" > /dev/null 2>&1

        # Check exit status
        if [ $? -eq 0 ]; then
            # Success: Write the data content to the text file
            rm -f "$TXT_FILE"
            printf "%s" "$DATA" > "$TXT_FILE"

            CHAR_COUNT=${#DATA}
            echo "Success! (Capacity: $CHAR_COUNT chars) -> Saved $TXT_FILE"
            break
        else
            # Failure: Data too long
            if [ -z "$RNDSTRING" ]; then
                echo "Failed: Version $n capacity too small."
                break
            fi

            # Remove last character and retry
            RNDSTRING="${RNDSTRING%?}"
        fi
    done
done

echo "---"
echo "Generation complete. Check '$OUTPUT_DIR' for .png and .txt files."
