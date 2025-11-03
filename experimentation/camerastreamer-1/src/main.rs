/// for windows/msys2/ucrt64 build (requires gnu toolchain):
/// `cargo build --target x86_64-pc-windows-gnu`
/// or:
/// `rustup run stable-x86_64-pc-windows-gnu cargo build --target x86_64-pc-windows-gnu`
///
/// run resulting binary only from msys2 environment, requires msys2 .dlls in the path.
///
/// for windows/vcpkg build using (requires msvc toolchain):
/// `cargo build --target x86_64-pc-windows-msvc`
/// or
/// `rustup run stable-x86_64-pc-windows-msvc cargo build --target x86_64-pc-windows-msvc --release`
///
/// Note: build script copies required dlls from vcpkg into the build directory next to the .exe
///
/// if you need a debug build use:
/// ```
/// set OPENCV_DISABLE_PROBES=vcpkg_cmake
/// rustup run stable-x86_64-pc-windows-msvc cargo build --target x86_64-pc-windows-msvc --release
/// ```
/// Reference: https://github.com/twistedfall/opencv-rust/issues/307
///
/// no other combinations tested.


// server.rs
use anyhow::Result;
use bytes::Bytes;
use opencv::{core, imgcodecs, prelude::*, videoio};
use std::sync::Arc;
use tokio::{
    net::TcpListener,
    sync::broadcast::{self, Sender},
    time::{self, Duration},
};

const ADDR: &str = "0.0.0.0:5000";
const WIDTH: i32 = 640;
const HEIGHT: i32 = 480;
const FPS: u32 = 20;
const BROADCAST_CAP: usize = 16;

#[tokio::main]
async fn main() -> Result<()> {
    // Create broadcast channel for frames (Arc<Bytes> so we cheaply clone for each client)
    let (tx, _rx) = broadcast::channel::<Arc<Bytes>>(BROADCAST_CAP);

    // Spawn capture loop
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        if let Err(e) = capture_loop(tx_clone).await {
            eprintln!("capture loop error: {e:?}");
        }
    });

    // Start listener
    let listener = TcpListener::bind(ADDR).await?;
    println!("Server listening on {}", ADDR);

    loop {
        let (socket, peer) = listener.accept().await?;
        println!("Client connected: {}", peer);
        let mut rx = tx.subscribe();
        tokio::spawn(async move {
            if let Err(e) = handle_client(socket, &mut rx).await {
                eprintln!("client {} error: {:?}", peer, e);
            } else {
                println!("client {} disconnected", peer);
            }
        });
    }
}

async fn capture_loop(tx: Sender<Arc<Bytes>>) -> Result<()> {
    // Open default camera (index 0)
    let mut cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)?; // 0 = default device
    if !videoio::VideoCapture::is_opened(&cam)? {
        anyhow::bail!("Unable to open default camera");
    }
    cam.set(videoio::CAP_PROP_FRAME_WIDTH, f64::from(WIDTH))?;
    cam.set(videoio::CAP_PROP_FRAME_HEIGHT, f64::from(HEIGHT))?;

    let period = Duration::from_millis((1000u32 / FPS) as u64);
    let mut interval = time::interval(period);
    loop {
        interval.tick().await;
        let mut frame = Mat::default();
        cam.read(&mut frame)?;
        if frame.empty() {
            // skip or try again
            continue;
        }

        // Encode to JPEG (quality default). You can set params to reduce quality/size.
        let mut buf = opencv::core::Vector::new();
        let params = opencv::core::Vector::new(); // default
        imgcodecs::imencode(".jpg", &frame, &mut buf, &params)?;

        // Wrap bytes into Arc so broadcast clones cheap
        let bytes = Arc::new(Bytes::from(buf.to_vec()));
        // Ignore send error (no subscribers)
        let _ = tx.send(bytes);
    }
}

async fn handle_client(mut socket: tokio::net::TcpStream, rx: &mut tokio::sync::broadcast::Receiver<Arc<Bytes>>) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    loop {
        // Receive latest frame (await)
        let bytes = match rx.recv().await {
            Ok(b) => b,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                // If lagged, try to get the next available
                continue;
            }
            Err(e) => return Err(anyhow::anyhow!(e)),
        };
        let len = (bytes.len() as u32).to_be_bytes();
        socket.write_all(&len).await?;
        socket.write_all(&bytes).await?;
    }
}
