@group(0) @binding(0)
var<storage, read_write> input: array<vec4<f32>>;

struct RGBSpaceParams{
	color_correction_matrix: mat4x4<f32>,
	gain: f32,
	gamma: f32,
}

var<push_constant> pc: RGBSpaceParams;

#import is_outside_image

@compute @workgroup_size(#WG_X, #WG_Y, #WG_Z)
fn main(
	@builtin(global_invocation_id) global_id: vec3<u32>,
){
	let global_bounds = vec2(#HEIGHT, #WIDTH);

	if is_outside_image(global_id, global_bounds){
		return;
	}

	let global_flat = i32(global_id.x) * #WIDTH + i32(global_id.y);

	var color = input[global_flat];
	color.w = 1.0;
	color = pc.color_correction_matrix * color;
	color = pc.gain * pow(color, vec4(pc.gamma));
	color.w = 1.0;

	input[global_flat] = color;
}
