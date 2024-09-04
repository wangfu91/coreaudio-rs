//! A basic input + output stream example, copying the mic input stream to the default output stream

extern crate coreaudio;

use std::sync::{Arc, Mutex};

use coreaudio::audio_unit::audio_format::LinearPcmFlags;
use coreaudio::audio_unit::macos_helpers::{get_default_device_id, vpio_audio_unit_from_device_id};
use coreaudio::audio_unit::render_callback::{self, data};
use coreaudio::audio_unit::{Element, SampleFormat, Scope, StreamFormat};
use coreaudio::sys::*;

const SAMPLE_RATE: f64 = 48000.0;

type S = f32;
const SAMPLE_FORMAT: SampleFormat = SampleFormat::F32;
// type S = i32; const SAMPLE_FORMAT: SampleFormat = SampleFormat::I32;
// type S = i16; const SAMPLE_FORMAT: SampleFormat = SampleFormat::I16;
// type S = i8; const SAMPLE_FORMAT: SampleFormat = SampleFormat::I8;

fn main() -> Result<(), coreaudio::Error> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let default_input_device_id = get_default_device_id(true).unwrap();
    let default_output_device_id = get_default_device_id(false).unwrap();

    let mut input_audio_unit =
        vpio_audio_unit_from_device_id(default_input_device_id, default_output_device_id)?;

    let format_flag = match SAMPLE_FORMAT {
        SampleFormat::F32 => LinearPcmFlags::IS_FLOAT,
        SampleFormat::I32 | SampleFormat::I16 | SampleFormat::I8 => {
            LinearPcmFlags::IS_SIGNED_INTEGER
        }
        _ => {
            unimplemented!("Other formats are not implemented for this example.");
        }
    };

    // Using IS_NON_INTERLEAVED everywhere because data::Interleaved is commented out / not implemented
    let in_stream_format = StreamFormat {
        sample_rate: SAMPLE_RATE,
        sample_format: SAMPLE_FORMAT,
        flags: format_flag | LinearPcmFlags::IS_PACKED | LinearPcmFlags::IS_NON_INTERLEAVED,
        // audio_unit.set_input_callback is hardcoded to 1 buffer, and when using non_interleaved
        // we are forced to 1 channel
        channels: 1,
    };

    println!("input={:#?}", &in_stream_format);
    println!("input_asbd={:#?}", &in_stream_format.to_asbd());

    let asbd = in_stream_format.to_asbd();
    input_audio_unit.set_property(
        kAudioUnitProperty_StreamFormat,
        Scope::Output,
        Element::Input,
        Some(&asbd),
    )?;

    input_audio_unit.set_property(
        kAudioUnitProperty_StreamFormat,
        Scope::Input,
        Element::Output,
        Some(&asbd),
    )?;

    type Args = render_callback::Args<data::NonInterleaved<S>>;

    let writer = hound::WavWriter::create("record.wav", spec).unwrap();
    let writer = Arc::new(Mutex::new(Some(writer)));
    // Run the input stream on a separate thread.
    let writer_2 = writer.clone();

    input_audio_unit.set_input_callback(move |args| {
        let Args {
            num_frames,
            mut data,
            ..
        } = args;
        // Print the number of frames the callback provides.
        // Included to aid understanding, don't use println and other things
        // that may block for an unknown amount of time inside the callback
        // of a real application.
        println!("input cb {} frames", num_frames);

        // Lock the WAV writer
        //let mut writer = writer_clone_for_callback.lock().unwrap();

        if let Ok(mut guard) = writer_2.try_lock() {
            if let Some(writer_3) = guard.as_mut() {
                for i in 0..num_frames {
                    for (_, channel) in data.channels_mut().enumerate() {
                        let value: S = channel[i];
                        writer_3.write_sample(value).unwrap();
                    }
                }
            }
        }

        Ok(())
    })?;

    input_audio_unit.initialize()?;

    input_audio_unit.start()?;

    println!("Input audio unit recording started");

    std::thread::sleep(std::time::Duration::from_millis(30 * 1000));

    writer.lock().unwrap().take().unwrap().finalize().unwrap();

    Ok(())
}
