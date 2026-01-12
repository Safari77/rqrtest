use clap::Parser;
use image::ImageReader;
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

use rqrr::DeQRError;
use rqrr::PreparedImage;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(value_name = "FILE")]
    file: PathBuf,

    #[arg(short, long, default_value_t = 1)]
    loops: u32,

    /// Disable debug logging for faster performance
    #[arg(long)]
    nodebug: bool,

    /// Enable standard QR code decoding (default: true if rmqr not specified)
    #[arg(long)]
    qr: bool,

    /// Enable rMQR code decoding (default: true if qr not specified)
    #[arg(long)]
    rmqr: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if args.nodebug {
        rqrr::debug::disable_debug();
    }

    // Determine execution mode
    // If no specific flag is provided, enable both.
    // If at least one is provided, enable only the specified ones.
    let (run_qr, run_rmqr) =
        if !args.qr && !args.rmqr { (true, true) } else { (args.qr, args.rmqr) };

    println!("Loading image from: {:?}", args.file);
    let img = ImageReader::open(&args.file)?.decode()?;
    let img_gray = img.to_luma8();

    println!(
        "Image loaded. Starting decode loop ({} iterations)...",
        args.loops
    );
    println!("Modes enabled: QR={}, rMQR={}", run_qr, run_rmqr);
    println!("---");

    let start_time = Instant::now();

    if run_qr {
        let mut prepared_img = PreparedImage::prepare(img_gray.clone());

        for i in 0..args.loops {
            let grids = prepared_img.detect_grids();

            if i == 0 {
                if grids.is_empty() {
                    println!("No QR codes detected.");
                } else {
                    for (idx, grid) in grids.into_iter().enumerate() {
                        match grid.decode() {
                            Ok((_meta, content)) => {
                                println!("Found QR Code #{}: {}", idx + 1, content)
                            }
                            Err(e) => {
                                if e != DeQRError::FormatEcc {
                                    eprintln!("Failed to decode QR candidate #{}: {:?}", idx + 1, e)
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if run_qr && run_rmqr {
        println!("\n\n");
    }

    if run_rmqr {
        let mut prepared_img = PreparedImage::prepare(img_gray.clone());
        let grids = prepared_img.detect_rmqr_grids();
        if grids.is_empty() {
            println!("No rMQR codes detected.");
        } else {
            for (idx, grid) in grids.into_iter().enumerate() {
                match grid.decode() {
                    Ok((_meta, content)) => println!("Found rMQR Code #{}: {}", idx + 1, content),
                    Err(e) => {
                        if e != DeQRError::FormatEcc {
                            eprintln!("Failed to decode rMQR candidate #{}: {:?}", idx + 1, e)
                        }
                    }
                }
            }
        }
    }

    let duration = start_time.elapsed();

    if args.loops > 1 {
        println!("---");
        println!("Performance Analysis:");
        println!("Total time: {:?}", duration);
        println!("Average time per decode: {:?}", duration / args.loops);
    }

    Ok(())
}
