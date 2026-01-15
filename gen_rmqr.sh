#!/bin/bash

# Create an output directory
OUTPUT_DIR="rmqr_versions"
mkdir -p "$OUTPUT_DIR"

echo "Starting rMQR Brute-force Generator (Versions 1-31)..."

for ecc in 0 4; do
    for n in {1..32}; do
        # 1. Generate initial random string
        RNDSTRING=$(randword 1 160 | tr -d '\n\r')
        if [[ $ecc == 0 ]]; then
            eccstr='M'
        else
            eccstr='H'
        fi

        echo -n "Processing Version $n ECC=${eccstr}... "

        while true; do
            DATA="VER$n $RNDSTRING"

            BASEFILE="${OUTPUT_DIR}/rmqr_v${n}_${eccstr}"
            # Define output filenames
            IMG_FILE="${BASEFILE}".png
            TXT_FILE="${BASEFILE}".txt

            rm -f "$IMG_FILE"
            # Run zint
            zint --quietzone --scale 10 --whitesp=1 --vwhitesp=1 \
                 --secure=$ecc -b rmqr --vers="$n" \
                  \
                 -d "$DATA" -o "$IMG_FILE" > /dev/null 2>&1
                 oxipng "$IMG_FILE" >& /dev/null

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
done

echo "---"
echo "Generation complete. Check '$OUTPUT_DIR' for .png and .txt files."
