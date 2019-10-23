use cpal::traits::*;

mod input_source;
mod led_serial;
mod window_buffer;
mod fft_processor;

mod context;

/// Data type to use in FFT and audio sampling.
///
/// Using anything other than `f32` will need some code changes.
type DataType = f32;

/// Channels of the input.
const CHANNELS: usize = 1;
const DATA_TYPE_CPAL: cpal::SampleFormat = cpal::SampleFormat::F32;
/// Sample rate of the input.
const SAMPLE_RATE: u32 = 44100;

/// Size of the window the FFT is run on.
const WINDOW_SIZE: usize = 2048;
/// After this many frames have been processed, the FFT is recalculated over the current window. 
const UPDATE_FRAMES: usize = WINDOW_SIZE / 4;
/// Size of one bin.
const BIN_SIZE: usize = 20;
/// Number of bins.
const BINS: usize = WINDOW_SIZE / BIN_SIZE;
/// Number of spectrum bins.
const SPECTRUM_BINS: usize = BINS / 2 - 1;


const DEFAULT_SERIAL_PORT: &'static str = "/dev/ttyUSB0";
const DEFAULT_DEVICE_INDEX: usize = 0;

fn prepare_input_stream(mut device_index: usize) -> (cpal::Host, cpal::EventLoop, cpal::StreamId) {
	let host = cpal::default_host();
	let event_loop = host.event_loop();

	let input_stream_id = {
		let fitting_device_indices = input_source::list_fitting_input_devices(&host, CHANNELS as u16, SAMPLE_RATE, DATA_TYPE_CPAL);
		if fitting_device_indices.len() == 0 {
			log::error!("Could not find any fitting device.");
			panic!("Could not find any fitting device.");
		}

		log::debug!("Fitting device indices:");
		for index in fitting_device_indices.iter() {
			log::debug!("\t{}", index);
		}

		if device_index >= fitting_device_indices.len() {
			log::warn!("Index {} is bigger than number of fitting devices ({}). Using 0.", device_index, fitting_device_indices.len());
			device_index = 0;
		}

		let device = host.devices().unwrap().nth(fitting_device_indices[device_index]).unwrap();
		let format = cpal::Format {
			channels: CHANNELS as u16,
			sample_rate: cpal::SampleRate(SAMPLE_RATE),
			data_type: DATA_TYPE_CPAL
		};
		log::info!("Using device {} with format {:?}.", device.name().unwrap(), format);

		let input_stream_id = event_loop.build_input_stream(&device, &format).expect("Could not build stream");
		event_loop.play_stream(input_stream_id.clone()).expect("Could not play stream");

		input_stream_id
	};

	(host, event_loop, input_stream_id)
}

fn main() {
	edwardium_logger::init(
		vec![
			edwardium_logger::StdoutTarget::new(
				log::Level::Trace, Default::default()
			)
		]
	).expect("Could not initialize logger");

	let args: Vec<String> = std::env::args().collect();
	
	let serial;
	let index: usize;
	if args.len() <= 1 {
		serial = DEFAULT_SERIAL_PORT;
		index = DEFAULT_DEVICE_INDEX;
	} else if args.len() <= 2 {
		serial = &args[1];
		index = DEFAULT_DEVICE_INDEX;
	} else {
		serial = &args[1];
		index = args[2].parse::<usize>().unwrap_or(DEFAULT_DEVICE_INDEX);
	}

	let (_host, event_loop, input_stream_id) = prepare_input_stream(index);

	let mut context = context::Context::new(serial);

	event_loop.run(move |id, result| {
		let data = match result {
			Ok(data) => data,
			Err(err) => {
				log::error!("An error occurred on stream {:?}: {}", id, err);
				return;
			}
		};

		match data {
			cpal::StreamData::Input { buffer: cpal::UnknownTypeInputBuffer::F32(buffer) } => {
				assert_eq!(id, input_stream_id);

				context.process_input_buffer(&buffer);
			},
			_ => panic!("expecting f32 input data"),
		}
	});
}
