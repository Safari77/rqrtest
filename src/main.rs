use clap::ArgGroup;
use clap::Parser;
use image::ImageReader;
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;

use rqrr::prepare::{BasicImageBuffer, PreparationConfig};
use rqrr::rmqr_detect;
use rqrr::rmqr_finder;
use rqrr::DeQRError;
use rqrr::PreparedImage;
use rqrr::RmqrGrid;
use rqrr::RmqrGridLocation;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(group(
    ArgGroup::new("binarizer")
        .args(["use_hybrid_binarizer", "use_adaptive"])
        .required(false)
        .multiple(false)
))]
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

    /// Threshold bias
    #[arg(short, long, default_value_t = 1)]
    threshold_bias: i16,

    /// RGB luminance bias weights (three floats in range 0.0–1.0)
    /// Example: --rgb-bias 0.299 0.587 0.114
    #[arg(
        long,
        value_name = "R G B",
        num_args = 3,
        value_parser = clap::value_parser!(f32)
    )]
    rgb_bias: Option<[f32; 3]>,

    /// Contrast strecth for low-contrast images
    #[arg(long, default_value_t = false)]
    contrast_stretch: bool,

    /// Use HybridBinarizer
    #[arg(long, default_value_t = false)]
    use_hybrid_binarizer: bool,

    /// Use adaptive thresholding
    #[arg(long, default_value_t = false)]
    use_adaptive: bool,

    /// Adaptive block radius
    #[arg(short, long, default_value_t = 50)]
    adaptive_block_radius: u32,

    #[arg(short, long, default_value_t = 10)]
    adaptive_threshold_delta: i32,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if args.nodebug {
        rqrr::debug::disable_debug();
    }
    if let Some(bias) = args.rgb_bias {
        if bias.iter().any(|&v| !(0.0..=1.0).contains(&v)) {
            return Err("rgb-bias values must be between 0.0 and 1.0".into());
        }
    }

    // Determine execution mode
    // If no specific flag is provided, enable both.
    // If at least one is provided, enable only the specified ones.
    let (run_qr, run_rmqr) =
        if !args.qr && !args.rmqr { (true, true) } else { (args.qr, args.rmqr) };

    let config = PreparationConfig {
        threshold_bias: args.threshold_bias,
        rgb_bias: args.rgb_bias.unwrap_or([0.299, 0.587, 0.114]),
        contrast_stretch: args.contrast_stretch,
        use_hybrid_binarizer: args.use_hybrid_binarizer,
        use_adaptive: args.use_adaptive,
        adaptive_block_radius: args.adaptive_block_radius,
        adaptive_threshold_delta: args.adaptive_threshold_delta,
        ..Default::default()
    };

    println!("Loading image from: {:?}", args.file);
    let img = ImageReader::open(&args.file)?.decode()?;
    let gray_img = PreparedImage::<BasicImageBuffer>::from_rgb(&img.to_rgb8(), config.clone());
    println!(
        "Image loaded. Starting decode loop ({} iterations)...",
        args.loops
    );
    println!("Modes enabled: QR={}, rMQR={}", run_qr, run_rmqr);
    println!("---");

    let start_time = Instant::now();

    if run_qr {
        let mut prepared_img = PreparedImage::prepare(gray_img.clone());

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
        let mut prepared_img = PreparedImage::prepare_with_config(gray_img.clone(), config);
        let capstones = rmqr_finder::find_rmqr_capstones(&prepared_img);
        let patterns = rmqr_detect::find_rmqr_patterns(&mut prepared_img, &capstones);

        // We scan for the sub-finder and verify format info here.
        let regions = rmqr_detect::match_rmqr_patterns(&prepared_img, &patterns);

        // 4. Convert Regions to Grids
        let mut grids = Vec::new();
        for region in regions {
            // We attempt to create a grid location from the region
            // Note: You might need to import RmqrGridLocation from your crate
            if let Some(grid_loc) = RmqrGridLocation::from_region(&prepared_img, &region) {
                let bounds = grid_loc.corners; // Save bounds for result
                let grid = grid_loc.into_grid_image(&prepared_img);
                grids.push(crate::RmqrGrid { grid, bounds });
            }
        }

        // 5. Decode
        if grids.is_empty() {
            println!("No rMQR codes detected.");
        } else {
            for (idx, grid) in grids.into_iter().enumerate() {
                match grid.decode() {
                    Ok((_meta, content)) => println!("Found rMQR Code #{}: {}", idx + 1, content),
                    Err(e) => {
                        // Ignore FormatEcc errors which are common in noise
                        if e != crate::DeQRError::FormatEcc {
                            eprintln!("Failed to decode rMQR candidate #{}: {:?}", idx + 1, e);
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
