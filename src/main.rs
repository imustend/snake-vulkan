use vulkano::{instance::{Instance, InstanceCreateInfo}, device::{physical::PhysicalDevice, Device, DeviceCreateInfo, QueueCreateInfo}, buffer::{CpuAccessibleBuffer, BufferUsage}, command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage}, sync::{self, GpuFuture}};
use bytemuck::{Pod, Zeroable};


#[repr(C)]
#[derive(Default, Clone, Copy, Zeroable, Pod)]
struct MyStruct {
	a: u32,
	b: u32,
}

fn main() {
	// creating vulkan instance 
	let instance = Instance::new(InstanceCreateInfo::default()).expect("failed to create vulkan instance");

	// choosing first device that supports vulkan
	// NOTE: this doesnt mean the best device, this should be user input, i think
	let physical = PhysicalDevice::enumerate(&instance).next().expect("no devices found");

	// displaying number of families and queues in those families 
	for family in physical.queue_families() {
		println!("found a family with {:?} queue(s)", family.queues_count());
	}

	// getting the queue family that supports graphical stuff
	let queue_family = physical.queue_families()
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
	let source = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, source_content)
	.expect("failed to create buffer");

	let destination_content: Vec<i32> = (0..64).map(|_| 0).collect();
	let destination = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, destination_content)
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
	builder.copy_buffer(source.clone(), destination.clone()).unwrap();

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

}