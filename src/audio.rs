use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use cpal::{
    Device, SampleFormat, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossbeam_channel::{Receiver, Sender, bounded};
use rustfft::{FftPlanner, num_complex::Complex};
use symphonia::core::{
    audio::{AudioBufferRef, Signal},
    codecs::DecoderOptions,
    errors::Error,
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};
use symphonia::default::get_probe;

/// Commands sent from the main application thread to the Audio Architecture Core
#[derive(Debug, Clone)]
pub enum AudioCommand {
    Play(PathBuf),
    _Stop,
    _Pause,
    _Resume,
    SetVolume(u8),      // 0 to 100
    _SetCrossfade(u32), // Seconds
}

pub struct AudioContext {
    _host: cpal::platform::Host,
    device: cpal::platform::Device,
    config: cpal::SupportedStreamConfig,
}

pub struct AudioEngine {
    _context: AudioContext,
    command_tx: Sender<AudioCommand>,
    fft_rx: Receiver<Vec<f32>>,
    _stream: Stream,
}

impl AudioContext {
    /// Creates a new AudioContext with default CPAL configuration
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize CPAL with proper error handling
        let host = cpal::default_host();

        // Get the default output device
        let device = host
            .default_output_device()
            .ok_or("No default output device available")?;

        // Get the default output configuration
        let config = device
            .default_output_config()
            .map_err(|e| format!("Failed to get default output config: {:?}", e))?;

        Ok(Self {
            config,
            _host: host,
            device: device,
        })
    }
}

impl AudioEngine {
    /// Spawns the internal audio background threads, sets up CPAL,
    /// and returns the engine controller handles.
    pub fn try_init() -> Result<Self, Box<dyn std::error::Error>> {
        // Create channels for communication
        let (command_tx, command_rx) = bounded(10);
        let (fft_tx, fft_rx) = bounded(1);

        // initialize CPAL context
        let context = AudioContext::new()?;
        let config = context.config.clone().into();

        // Create the audio stream
        let stream = match context.config.sample_format() {
            SampleFormat::F32 => {
                Self::build_stream::<f32>(&context.device, &config, command_rx, fft_tx)?
            }
            SampleFormat::I16 => {
                Self::build_stream::<i16>(&context.device, &config, command_rx, fft_tx)?
            }
            SampleFormat::U16 => {
                Self::build_stream::<u16>(&context.device, &config, command_rx, fft_tx)?
            }
            sample_format => {
                return Err(format!("Unsupported sample format: '{sample_format}'").into());
            }
        };

        Ok(Self {
            _context: context,
            command_tx,
            fft_rx,
            _stream: stream,
        })
    }

    /// Non-blocking method to submit commands to the background audio runner
    pub fn send_command(&self, cmd: AudioCommand) {
        let _ = self.command_tx.send(cmd);
    }

    /// Non-blocking check to grab the latest FFT frequency bins for the visualizer
    pub fn try_recv_fft(&self) -> Option<Vec<f32>> {
        self.fft_rx.try_recv().ok()
    }

    fn build_stream<T>(
        device: &Device,
        config: &StreamConfig,
        command_rx: Receiver<AudioCommand>,
        fft_tx: Sender<Vec<f32>>,
    ) -> Result<Stream, Box<dyn std::error::Error>>
    where
        T: cpal::Sample + cpal::SizedSample + rustfft::num_traits::Zero + cpal::FromSample<f32>,
    {
        // Create shared state
        let shared_state = Arc::new(Mutex::new(SharedState::new()));

        // Spawn the decoder thread
        let decoder_state = shared_state.clone();
        thread::spawn(move || {
            Self::decoder_thread(command_rx, decoder_state);
        });

        // Build the stream
        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                Self::audio_callback(data, &shared_state, &fft_tx);
            },
            |err| eprintln!("CPAL error: {}", err),
            None,
        )?;

        stream.play()?;
        Ok(stream)
    }

    fn decoder_thread(command_rx: Receiver<AudioCommand>, shared_state: Arc<Mutex<SharedState>>) {
        let mut volume = 1.0; // Default volume (100%)

        while let Ok(cmd) = command_rx.recv() {
            match cmd {
                AudioCommand::Play(path) => {
                    if let Err(e) = Self::decode_and_queue(&path, &shared_state, volume) {
                        eprintln!("Error decoding audio: {:?}", e);
                    }
                }
                AudioCommand::_Stop => {
                    let mut state = shared_state.lock().unwrap();
                    state.playback_buffer.clear();
                    state.is_playing = false;
                }
                AudioCommand::_Pause => {
                    let mut state = shared_state.lock().unwrap();
                    state.is_playing = false;
                }
                AudioCommand::_Resume => {
                    let mut state = shared_state.lock().unwrap();
                    state.is_playing = true;
                }
                AudioCommand::SetVolume(level) => {
                    volume = level as f32 / 100.0;
                    let mut state = shared_state.lock().unwrap();
                    state.volume = volume;
                }
                AudioCommand::_SetCrossfade(_) => {
                    // Crossfade implementation would go here
                }
            }
        }
    }

    fn decode_and_queue(
        path: &PathBuf,
        shared_state: &Arc<Mutex<SharedState>>,
        volume: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Open the media source
        let src = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(src), Default::default());

        // Create the format reader
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            hint.with_extension(ext);
        }

        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        let probed = get_probe().format(&hint, mss, &fmt_opts, &meta_opts)?;
        let mut format = probed.format;

        // Get the default track
        let (track_id, codec_params) = {
            let t = format.default_track().ok_or("No default track")?;
            (t.id, t.codec_params.clone())
        };

        // Create a decoder for the track
        let dec_opts: DecoderOptions = Default::default();
        let mut decoder = symphonia::default::get_codecs()
            .make(&codec_params, &dec_opts)
            .map_err(|e| format!("Failed to create decoder: {:?}", e))?;

        // Decode packets and queue samples
        let mut state = shared_state.lock().unwrap();
        state.playback_buffer.clear();
        state.is_playing = true;

        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(Error::ResetRequired) => {
                    // Handle reset if needed
                    continue;
                }
                Err(e) => {
                    eprintln!("Error reading packet: {:?}", e);
                    break;
                }
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(audio_buf) => match audio_buf {
                    AudioBufferRef::F32(buf) => {
                        for sample in buf.chan(0) {
                            state.playback_buffer.push(*sample * volume);
                        }
                    }
                    _ => {
                        eprintln!("Unsupported audio buffer type");
                        continue;
                    }
                },
                Err(e) => {
                    eprintln!("Error decoding packet: {:?}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    fn audio_callback<T>(
        output: &mut [T],
        shared_state: &Arc<Mutex<SharedState>>,
        fft_tx: &Sender<Vec<f32>>,
    ) where
        T: cpal::Sample + cpal::SizedSample + rustfft::num_traits::Zero + cpal::FromSample<f32>,
    {
        let mut state = shared_state.lock().unwrap();

        if !state.is_playing || state.playback_buffer.is_empty() {
            for sample in output.iter_mut() {
                *sample = T::zero();
            }
            return;
        }

        let samples_needed = output.len();
        let mut samples_written = 0;

        while samples_written < samples_needed && !state.playback_buffer.is_empty() {
            let sample = state.playback_buffer.remove(0);
            let converted_sample = cpal::Sample::from_sample::<f32>(sample);
            output[samples_written] = converted_sample;
            samples_written += 1;

            // Update the visualizer window
            state.visualizer_window.push(sample);
            if state.visualizer_window.len() > 512 {
                state.visualizer_window.remove(0);
            }
        }

        // Pad remaining samples with zeros if needed
        for sample in output.iter_mut().skip(samples_written) {
            *sample = T::zero();
        }

        // Compute FFT if we have enough samples
        if state.visualizer_window.len() == 512 {
            let mut planner = FftPlanner::new();
            let fft = planner.plan_fft_forward(512);

            let mut buffer: Vec<Complex<f32>> = state
                .visualizer_window
                .iter()
                .map(|&x| Complex::new(x, 0.0))
                .collect();

            // Apply window function (Hann window)
            for (i, sample) in buffer.iter_mut().enumerate() {
                let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / 511.0).cos());
                *sample *= Complex::new(window, 0.0);
            }

            fft.process(&mut buffer);

            // Calculate magnitude spectrum
            let mut magnitudes: Vec<f32> = buffer
                .iter()
                .take(256) // Only take the first half (Nyquist frequency)
                .map(|c| c.norm())
                .collect();

            // Normalize the magnitudes
            let max_magnitude = magnitudes.iter().fold(0.0f32, |a, &b| a.max(b));
            if max_magnitude > 0.0 {
                for magnitude in magnitudes.iter_mut() {
                    *magnitude /= max_magnitude;
                }
            }

            // Send the FFT data to the UI thread
            let _ = fft_tx.send(magnitudes);
        }
    }
}

struct SharedState {
    playback_buffer: Vec<f32>,
    visualizer_window: Vec<f32>,
    is_playing: bool,
    volume: f32,
}

impl SharedState {
    fn new() -> Self {
        Self {
            playback_buffer: Vec::new(),
            visualizer_window: Vec::new(),
            is_playing: false,
            volume: 1.0,
        }
    }
}
