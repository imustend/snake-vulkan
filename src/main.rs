use std::future;

use bytemuck::{Pod, Zeroable};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, self},
    device::{physical::PhysicalDevice, Device, DeviceCreateInfo, QueueCreateInfo},
    instance::{Instance, InstanceCreateInfo},
    sync::{self, GpuFuture}, pipeline::{ComputePipeline, Pipeline, PipelineBindPoint}, descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
};

#[repr(C)]
#[derive(Default, Clone, Copy, Zeroable, Pod)]
struct MyStruct {
    a: u32,
    b: u32,
}

fn main() {
    // creating vulkan instance
    let instance =
        Instance::new(InstanceCreateInfo::default()).expect("failed to create vulkan instance");

    // choosing first device that supports vulkan
    // NOTE: this doesnt mean the best device, this should be user input, i think
    let physical = PhysicalDevice::enumerate(&instance)
        .next()
        .expect("no devices found");

    println!("device: {}", physical.properties().device_name);

    // displaying number of families and queues in those families
    for family in physical.queue_families() {
        println!("found a family with {:?} queue(s)", family.queues_count());
    }

    // getting the queue family that supports graphical stuff
    let queue_family = physical
        .queue_families()
        .find(|&q| q.supports_graphics())
        .expect("couldnt find a graphical queue family");

    // creating the device and getting the queues to comunicate with the gpu
    let (device, mut queues) = Device::new(
        physical,
        DeviceCreateInfo {
            queue_create_infos: vec![QueueCreateInfo::family(queue_family)],
            ..Default::default()
        },
    )
    .expect("failed to create decive");

    // getting a single queue
    //NOTE: from what i understand this is also not the proper way to do this stuff
    let queue = queues.next().unwrap();

    // BUFFER STUFF
    // from what i understand buffer is place in memory where CPU and GPU can communicate
    // they both write and read from there

    // creating simple buffer
    // NOTE: i could use the cpu cached buffer with the whole snake struct in it
    let data = MyStruct { a: 5, b: 69 };
    let buffer = CpuAccessibleBuffer::from_data(device.clone(), BufferUsage::all(), false, data)
        .expect("failed to create buffer");

    // getting the struct to write to
    let mut contents = buffer.write().unwrap();
    // `content` implements `DerefMut whose target is of type `MyStruct` (the content of the buffer)
    contents.a *= 2;
    contents.b = 9;

    // FIRST GPU COMPUTATION
    // will copy data from one buffer to another

    // creating the buffers
    let source_content: Vec<i32> = (0..64).collect();
    let source =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, source_content)
            .expect("failed to create buffer");

    let destination_content: Vec<i32> = (0..64).map(|_| 0).collect();
    let destination = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::all(),
        false,
        destination_content,
    )
    .expect("failed to create buffer");

    // creating builder(?) for command buffer(?)
    // i have to read that chapter one more time
    let mut builder = AutoCommandBufferBuilder::primary(
        device.clone(),
        queue.family(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();

    // adding command to command buffer(?)
    builder
        .copy_buffer(source.clone(), destination.clone())
        .unwrap();

    let command_buffer = builder.build().unwrap();

    // syncing cpu with gpu and sending command buffer and executing it
    let future = sync::now(device.clone())
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap();

    future.wait(None).unwrap();

    let src_content = source.read().unwrap();
    let destination_content = destination.read().unwrap();
    assert_eq!(&*src_content, &*destination_content);

    //
    // COMPUTE PIPELINE

	// creating the buffer
    let data_iter = 0..65536;
    let data_buffer =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, data_iter)
            .expect("failed to create buffer");
    {}

	// creating the shader module the shader
    mod cs {
        vulkano_shaders::shader! {
            ty: "compute",
            src: "
		#version 450
		
		layout(local_size_x = 64, local_size_y = 1, local_size_z = 1) in;
		
		layout(set = 0, binding = 0) buffer Data {
			uint data[];
		} buf;
		
		void main() {
			uint idx = gl_GlobalInvocationID.x;
			buf.data[idx] *= 12;
		}"
        }
    }

	// compiling the shader
	let shader = cs::load(device.clone())
		.expect("could not load shader");

	// creating compute pipeline 
	let compute_pipeline = ComputePipeline::new(
		device.clone(), 
		shader.entry_point("main").unwrap(), 
		&(), 
		None, 
		|_| {}
	).expect("failed to create compute pipeline");


	// adding buffer to pipeline, somehow
	// here 0 means set
	let layout = compute_pipeline.layout().set_layouts().get(0).unwrap();

	let set = PersistentDescriptorSet::new(
		layout.clone(), 
		// here 0 means binding
		[WriteDescriptorSet::buffer(0, data_buffer.clone())],
	).unwrap();

	let mut builder = AutoCommandBufferBuilder::primary(
		device.clone(), 
		queue.family(),
		CommandBufferUsage::OneTimeSubmit
	).unwrap();

	builder
		.bind_pipeline_compute(compute_pipeline.clone()).
		bind_descriptor_sets(
			PipelineBindPoint::Compute,
			compute_pipeline.layout().clone(), 
			0, 
			set
		)
		.dispatch([1024, 1, 1])
		.unwrap();

	let command_buffer = builder.build().unwrap();


	let future = sync::now(device.clone())
		.then_execute(queue.clone(), command_buffer)
		.unwrap()
		.then_signal_fence_and_flush()
		.unwrap();
	
	future.wait(None).unwrap();

	let content = data_buffer.read().unwrap();
	for (n, val) in content.iter().enumerate() {
		assert_eq!(*val, n as u32 * 12);
	}

	println!("Everything succeeded!");
}
