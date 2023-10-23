@group(0) @binding(0)
var<storage, read> input: array<f32>;

@group(0) @binding(1)
var<storage, read_write> output: array<vec4<f32>>;

var<workgroup> local: array<f32, #expr{(WG_X * 2) * (WG_Y * 2)}>;

const local_height = #expr{WG_X * 2};
const local_width = #expr{WG_Y * 2};
const local_size = #expr{(WG_X * 2) * (WG_Y * 2)};

#import reflect_vec
#import is_outside_image
#import access_local

fn setup_local(wg_id: vec3<u32>, local_index: u32, global_bounds: vec2<i32>){
	let offset_to_global = vec2<i32>(wg_id.xy) * vec2(i32(#WG_X), i32(#WG_Y)) * 2;

	var local_flat = i32(local_index);

	while local_flat < local_size{
		let local_coord = vec2(
			i32((local_flat) / local_width),
			i32((local_flat) % local_width),
		);
		let global_coord = reflect_vec(local_coord + offset_to_global, global_bounds*2);

		let global_flat = global_coord.x * #WIDTH + global_coord.y;
		local[local_flat] = input[global_flat];

		local_flat += i32(#WG_X * #WG_Y);
	}
	workgroupBarrier();
}

@compute @workgroup_size(#WG_X, #WG_Y, #WG_Z)
fn main(
	@builtin(global_invocation_id) global_id: vec3<u32>,
	@builtin(local_invocation_id) local_id: vec3<u32>,
	@builtin(local_invocation_index) local_index: u32,
	@builtin(workgroup_id) wg_id: vec3<u32>,
){
	let global_bounds = vec2(#HEIGHT / 2, #WIDTH / 2);

	setup_local(wg_id, local_index, global_bounds);
	if is_outside_image(global_id, global_bounds){
		return;
	}

	var color: vec4<f32>;

	let double_local = vec2<i32>(local_id.xy) * 2;

	color.x = access_local(double_local.x + 0, double_local.y + 0);
	color.y = access_local(double_local.x + 0, double_local.y + 1);
	color.z = access_local(double_local.x + 1, double_local.y + 0);
	color.w = access_local(double_local.x + 1, double_local.y + 1);

	let global_flat = i32(global_id.x) * global_bounds.y + i32(global_id.y);
	output[global_flat] = color;
}
