use anyhow::Result;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_rtsp_server as gst_rtsp;
use gstreamer_rtsp_server::prelude::*;
use log::{info, error};

pub struct RtspServer {
    port: u16,
}

impl RtspServer {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    pub fn run(&self) -> Result<()> {
        // Initialize GStreamer
        gst::init()?;
        
        info!("Creating RTSP server on port {}", self.port);

        let main_loop = gst::glib::MainLoop::new(None, false);
        let server = gst_rtsp::RTSPServer::new();
        
        // Set the server port
        let service = self.port.to_string();
        server.set_service(&service);

        // Get the mount points
        let mounts = server.mount_points().unwrap();
        let factory = gst_rtsp::RTSPMediaFactory::new();

        // Create a pipeline that captures video from a camera
        // This is a generic pipeline that works with various camera sources
        // Adjust the source element based on your actual camera setup:
        // - v4l2src for USB/CSI cameras on Linux
        // - rpicamsrc for Raspberry Pi camera
        // - videotestsrc for testing without a real camera
        let pipeline_str = concat!(
            "( v4l2src device=/dev/video0 ! ",
            "video/x-raw,width=640,height=480,framerate=30/1 ! ",
            "videoconvert ! ",
            "x264enc tune=zerolatency bitrate=500 speed-preset=superfast ! ",
            "rtph264pay name=pay0 pt=96 )"
        );

        factory.set_launch(pipeline_str);
        factory.set_shared(true);

        // Mount the factory at /stream
        mounts.add_factory("/stream", factory);

        // Attach the server to the default main context
        let id = server.attach(None)?;
        
        info!("RTSP server ready at rtsp://0.0.0.0:{}/stream", self.port);
        info!("Server attach id: {}", id);

        // Run the main loop
        main_loop.run();

        Ok(())
    }
}
