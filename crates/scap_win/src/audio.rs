use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use glimpse_core::frame::{AudioFormat, AudioFrame, Frame};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, SystemTime};

pub fn audio_format_from_cpal(value: cpal::SampleFormat) -> AudioFormat {
    match value {
        cpal::SampleFormat::F32 => AudioFormat::F32,
        cpal::SampleFormat::F64 => AudioFormat::F64,
        cpal::SampleFormat::I8 => AudioFormat::I8,
        cpal::SampleFormat::I16 => AudioFormat::I16,
        cpal::SampleFormat::I32 => AudioFormat::I32,
        cpal::SampleFormat::I64 => AudioFormat::I64,
        cpal::SampleFormat::U8 => AudioFormat::U8,
        cpal::SampleFormat::U16 => AudioFormat::U16,
        cpal::SampleFormat::U32 => AudioFormat::U32,
        cpal::SampleFormat::U64 => AudioFormat::U64,
        _ => panic!("sample format {value:?} not supported"),
    }
}

pub struct AudioStreamHandle {
    pub ctrl_tx: mpsc::Sender<AudioStreamControl>,
}

#[derive(Debug)]
pub enum AudioStreamControl {
    Start,
    Stop,
}

#[derive(Debug)]
pub enum CreateAudioError {
    AudioStreamConfig(cpal::DefaultStreamConfigError),
    BuildAudioStream(cpal::BuildStreamError),
}

type AudioSample = Result<(Vec<u8>, cpal::InputCallbackInfo, SystemTime), cpal::StreamError>;

fn build_audio_stream(
    sample_tx: mpsc::Sender<AudioSample>,
) -> Result<(cpal::Stream, cpal::SupportedStreamConfig), CreateAudioError> {
    let host = cpal::default_host();
    let output_device = host
        .default_output_device()
        .ok_or(CreateAudioError::AudioStreamConfig(
            cpal::DefaultStreamConfigError::DeviceNotAvailable,
        ))?;
    let supported_config = output_device
        .default_output_config()
        .map_err(CreateAudioError::AudioStreamConfig)?;
    let config = supported_config.clone().into();

    let stream = output_device
        .build_input_stream_raw(
            &config,
            supported_config.sample_format(),
            {
                let sample_tx = sample_tx.clone();
                move |data, info: &cpal::InputCallbackInfo| {
                    sample_tx
                        .send(Ok((data.bytes().to_vec(), *info, SystemTime::now())))
                        .unwrap();
                }
            },
            move |e| {
                let _ = sample_tx.send(Err(e));
            },
            None,
        )
        .map_err(CreateAudioError::BuildAudioStream)?;

    Ok((stream, supported_config))
}

pub fn spawn_audio_stream(
    tx: Sender<Frame>,
    ready_tx: Sender<Result<(), CreateAudioError>>,
    ctrl_rx: Receiver<AudioStreamControl>,
) {
    std::thread::spawn(move || {
        let (sample_tx, sample_rx) = mpsc::channel();

        let res = build_audio_stream(sample_tx);

        let (stream, config) = match res {
            Ok(stream) => {
                let _ = ready_tx.send(Ok(()));
                stream
            }
            Err(e) => {
                let _ = ready_tx.send(Err(e));
                return;
            }
        };

        let Ok(ctrl) = ctrl_rx.recv() else {
            return;
        };

        match ctrl {
            AudioStreamControl::Start => {
                stream.play().unwrap();
            }
            AudioStreamControl::Stop => {
                return;
            }
        }

        let audio_format = audio_format_from_cpal(config.sample_format());

        loop {
            match ctrl_rx.try_recv() {
                Ok(AudioStreamControl::Stop) => return,
                Ok(_) | Err(mpsc::TryRecvError::Empty) => {}
                Err(_) => return,
            };

            let (data, _info, timestamp) = match sample_rx.recv_timeout(Duration::from_millis(100))
            {
                Ok(Ok(data)) => data,
                Err(RecvTimeoutError::Timeout) => {
                    continue;
                }
                _ => {
                    return;
                }
            };

            let sample_count =
                data.len() / (audio_format.sample_size() * config.channels() as usize);
            let frame = AudioFrame::new(
                audio_format,
                config.channels(),
                false,
                data,
                sample_count,
                config.sample_rate(),
                timestamp,
            );

            if tx.send(Frame::Audio(frame)).is_err() {
                return;
            };
        }
    });
}
