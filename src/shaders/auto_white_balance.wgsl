@group(0) @binding(0)
var<storage, read> input: array<f32>;

@group(0) @binding(1)
var<storage, read_write> output: array<f32>;

@group(0) @binding(2)
var<storage, read> mean: vec4<f32>;

struct AutoWhiteBalanceParams{
	gain: f32,
}

var<push_constant> pc: AutoWhiteBalanceParams;

#import is_outside_image

@compute @workgroup_size(#WG_X, #WG_Y, #WG_Z)
fn main(
	@builtin(global_invocation_id) global_id: vec3<u32>,
){
	let global_bounds = vec2(#HEIGHT, #WIDTH);

	if is_outside_image(global_id, global_bounds){
		return;
	}

	let mod_row = global_id.x % 2u;
	let mod_col = global_id.y % 2u;

	let global_flat = i32(global_id.x) * #WIDTH + i32(global_id.y);

	var color = input[global_flat];
	
	// Red
	if mod_row == 0u && mod_col == 0u{
		let red_avg = mean.x;
		let green_avg = (mean.y + mean.z) / 2.;
		color *= pc.gain * red_avg / green_avg;
	
	// Green
	} else if (mod_row == 0u && mod_col == 1u) || (mod_row == 1u && mod_col == 0u){
		color *= pc.gain;
		
	// Blue
	} else {
		let blue_avg = mean.w;
		let green_avg = (mean.y + mean.z) / 2.;
		color *= pc.gain * blue_avg / green_avg;
	}

	output[global_flat] = color;
}


