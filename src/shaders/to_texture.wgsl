
@group(0) @binding(0)
var<storage, read> buffer: array<vec4<f32>>;

@group(0) @binding(1)
var texture: texture_storage_2d<rgba32float, write>;

#import is_outside_image

@compute @workgroup_size(#WG_X, #WG_Y, #WG_Z)
fn main(
	@builtin(global_invocation_id) global_id: vec3<u32>,
){
	let global_bounds = vec2(#HEIGHT, #WIDTH);
	
	if is_outside_image(global_id, global_bounds){
		return;
	}
	let global_flat = global_id.x * u32(#WIDTH) + global_id.y;


	let load = buffer[global_flat];
	var rgb = load.rgb;
	rgb /= 1200.;
	rgb = pow(rgb, vec3(2.0));
	
	textureStore(texture, global_id.yx, vec4(rgb, load.w));
}
