@group(0) @binding(0)
var<storage, read> input: array<f32>;

@group(0) @binding(1)
var<storage, read_write> output: array<f32>;

struct BlackLevelParams{
	r_offset: f32,
	gr_offset: f32,
	gb_offset: f32,
	b_offset: f32,
	alpha: f32,
	beta: f32,
}

var<push_constant> pc: BlackLevelParams;

var<workgroup> local: array<f32, #expr{(WG_X + 2 * PADDING) * (WG_Y + 2 * PADDING)}>;

#import all_utils

const local_height = #expr{WG_X + 2 * PADDING};
const local_width = #expr{WG_Y + 2 * PADDING};
const local_size = #expr{(WG_X + 2 * PADDING) * (WG_Y + 2 * PADDING)};

@compute @workgroup_size(#WG_X, #WG_Y, #WG_Z)
fn main(
	@builtin(global_invocation_id) global_id: vec3<u32>,
	@builtin(local_invocation_id) local_id: vec3<u32>,
	@builtin(local_invocation_index) local_index: u32,
	@builtin(workgroup_id) wg_id: vec3<u32>,
){
	let global_bounds = vec2(#HEIGHT, #WIDTH);

	setup_local(wg_id, local_index, global_bounds);
	
	if is_outside_image(global_id, global_bounds){
		return;
	}

	let local_center = vec2<i32>(local_id.xy) + vec2(#PADDING);
	
	let mod_row = global_id.x % 2u;
	let mod_col = global_id.y % 2u;

	var new_val = 0.0;
	
	// Red
	if mod_row == 0u && mod_col == 0u{
		new_val = access_local(local_center.x, local_center.y) + pc.r_offset;
	
	// Green (red)
	} else if mod_row == 0u && mod_col == 1u {
		new_val = access_local(local_center.x, local_center.y) +
		pc.gr_offset +
		pc.alpha * access_local(local_center.x, local_center.y - 1);
		
	// Green (blue)
	} else if mod_row == 1u && mod_col == 0u {
		new_val = access_local(local_center.x, local_center.y) +
		pc.gb_offset +
		pc.beta * access_local(local_center.x - 1, local_center.y);

	// Blue
	} else {
		new_val = access_local(local_center.x, local_center.y) + pc.b_offset;
	}

	let global_flat = (i32(global_id.x) * #WIDTH + i32(global_id.y));

	output[global_flat] = new_val;
}

