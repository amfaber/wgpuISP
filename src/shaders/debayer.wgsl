@group(0) @binding(0)
var<storage, read> input: array<f32>;

@group(0) @binding(1)
var<storage, read_write> output: array<vec4<f32>>;

var<workgroup> local: array<f32, #expr{(WG_X + 2 * PADDING) * (WG_Y + 2 * PADDING)}>;

#import all_utils

const local_height = #expr{WG_X + 2 * PADDING};
const local_width = #expr{WG_Y + 2 * PADDING};
const local_size = #expr{(WG_X + 2 * PADDING) * (WG_Y + 2 * PADDING)};

fn malvar_r(center: vec2<i32>) -> vec3<f32>{
	let r = access_local(center.x, center.y);
	let g = (
		4.0 * access_local(center.x, center.y) -
		access_local(center.x - 2, center.y) -
		access_local(center.x, center.y - 2) -
		access_local(center.x + 2, center.y) -
		access_local(center.x, center.y + 2) +
		2. * (
			access_local(center.x + 1, center.y) +
			access_local(center.x, center.y + 1) +
			access_local(center.x - 1, center.y) +
			access_local(center.x, center.y - 1)
		)
	) / 8.0;
	
    let b = (
		6.0 * access_local(center.x, center.y) -
		3.0 * (
			access_local(center.x - 2, center.y) +
			access_local(center.x, center.y - 2) +
			access_local(center.x + 2, center.y) +
			access_local(center.x, center.y + 2)
		) / 2.0 +
		2.0 * (
			access_local(center.x - 1, center.y - 1) +
			access_local(center.x - 1, center.y + 1) +
			access_local(center.x + 1, center.y - 1) +
			access_local(center.x + 1, center.y + 1)
		)
	) / 8.;

	return vec3(r, g, b);
}

fn malvar_gr(center: vec2<i32>) -> vec3<f32>{
	let r = (
		5.0 * access_local(center.x, center.y) -
		access_local(center.x, center.y - 2) -
		access_local(center.x - 1, center.y - 1) -
		access_local(center.x + 1, center.y - 1) -
		access_local(center.x - 1, center.y + 1) -
		access_local(center.x + 1, center.y + 1) -
		access_local(center.x, center.y + 2) +
		(
			access_local(center.x - 2, center.y) +
			access_local(center.x + 2, center.y)
		) / 2.0 +
		4.0 * (
			access_local(center.x, center.y - 1) +
			access_local(center.x, center.y + 1)
		)
	) / 8.0;
	
	let g = access_local(center.x, center.y);
	
    let b = (
		5.0 * access_local(center.x, center.y) -
		access_local(center.x - 2, center.y) -
		access_local(center.x - 1, center.y - 1) -
		access_local(center.x - 1, center.y + 1) -
		access_local(center.x + 2, center.y) -
		access_local(center.x + 1, center.y - 1) -
		access_local(center.x + 1, center.y + 1) +
		(
			access_local(center.x, center.y - 2) +
			access_local(center.x, center.y + 2)
		) / 2.0 +
		4.0 * (
			access_local(center.x - 1, center.y) +
			access_local(center.x + 1, center.y)
		)
	) / 8.0;

	return vec3(r, g, b);
}

fn malvar_gb(center: vec2<i32>) -> vec3<f32>{
    let r = (
		5.0 * access_local(center.x, center.y) -
		access_local(center.x - 2, center.y) -
		access_local(center.x - 1, center.y - 1) -
		access_local(center.x - 1, center.y + 1) -
		access_local(center.x + 2, center.y) -
		access_local(center.x + 1, center.y - 1) -
		access_local(center.x + 1, center.y + 1) +
		(
			access_local(center.x, center.y - 2) +
			access_local(center.x, center.y + 2)
		) / 2.0 +
		4.0 * (
			access_local(center.x - 1, center.y) +
			access_local(center.x + 1, center.y)
		)
	) / 8.0;

	let g = access_local(center.x, center.y);

	let b = (
		5.0 * access_local(center.x, center.y) -
		access_local(center.x, center.y - 2) -
		access_local(center.x - 1, center.y - 1) -
		access_local(center.x + 1, center.y - 1) -
		access_local(center.x - 1, center.y + 1) -
		access_local(center.x + 1, center.y + 1) -
		access_local(center.x, center.y + 2) + (
			access_local(center.x - 2, center.y) +
			access_local(center.x + 2, center.y)
		) / 2.0 + 4.0 * (
			access_local(center.x, center.y - 1) +
			access_local(center.x, center.y + 1)
		)) / 8.0;

	return vec3(r, g, b);
}

fn malvar_b(center: vec2<i32>) -> vec3<f32>{
	let r = (
		6.0 * access_local(center.x, center.y) -
		3.0 * (
			access_local(center.x - 2, center.y) +
			access_local(center.x, center.y - 2) +
			access_local(center.x + 2, center.y) +
			access_local(center.x, center.y + 2)
		) / 2.0 +
		2.0 * (
			access_local(center.x - 1, center.y - 1) +
			access_local(center.x - 1, center.y + 1) +
			access_local(center.x + 1, center.y - 1) +
			access_local(center.x + 1, center.y + 1)
		)
	) / 8.0;

	let g = (
		4.0 * access_local(center.x, center.y) -
		access_local(center.x - 2, center.y) -
		access_local(center.x, center.y - 2) -
		access_local(center.x + 2, center.y) -
		access_local(center.x, center.y + 2) +
		2.0 * (
			access_local(center.x + 1, center.y) +
			access_local(center.x, center.y + 1) +
			access_local(center.x - 1, center.y) +
			access_local(center.x, center.y - 1)
		)
	) / 8.0;

	let b = access_local(center.x, center.y);
	return vec3(r, g, b);
}


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
	
	var color: vec3<f32>;

	let mod_row = global_id.x % 2u;
	let mod_col = global_id.y % 2u;

	if mod_row == 0u && mod_col == 0u{
		// color = malvar_r(local_center);
		color = vec3(access_local(local_center.x, local_center.y), 0., 0.);
	} else if mod_row == 0u && mod_col == 1u {
		// color = malvar_gr(local_center);
		color = vec3(0., access_local(local_center.x, local_center.y), 0.);
	} else if mod_row == 1u && mod_col == 0u {
		// color = malvar_gb(local_center);
		color = vec3(0., access_local(local_center.x, local_center.y), 0.);
	} else {
		// color = malvar_b(local_center);
		color = vec3(0., 0., access_local(local_center.x, local_center.y));
	}

	let global_flat = (i32(global_id.x) * #WIDTH + i32(global_id.y));

	output[global_flat] = vec4(color, 1.);
}

