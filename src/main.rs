use clap::Parser;
use image::ImageReader;
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

use rqrr::PreparedImage;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(value_name = "FILE")]
    file: PathBuf,

    #[arg(short, long, default_value_t = 1)]
    loops: u32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    //rqrr::debug::disable_debug(); // 15% faster
    println!("Loading image from: {:?}", args.file);
    let img = ImageReader::open(&args.file)?.decode()?;
    let img_gray = img.to_luma8();

    println!(
        "Image loaded. Starting decode loop ({} iterations)...",
        args.loops
    );
    println!("---");

    let start_time = Instant::now();

    for i in 0..args.loops {
        let mut prepared_img = PreparedImage::prepare(img_gray.clone());
        let grids = prepared_img.detect_grids();

        if i == 0 {
            if grids.is_empty() {
                println!("No QR codes detected.");
            } else {
                for (idx, grid) in grids.into_iter().enumerate() {
                    let (_meta, content) = grid.decode()?;
                    println!("Found QR Code #{}: {}", idx + 1, content);
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
