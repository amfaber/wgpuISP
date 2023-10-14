@group(0) @binding(0)
var<storage, read> input: array<#TY>;

@group(0) @binding(1)
var<storage, read_write> output: array<#TY>;

struct PushConstants{
	width: i32,
	height: i32,
}

var<push_constant> pc: PushConstants;

@compute @workgroup_size(#WG_X, #WG_Y, #WG_Z)
fn main(@builtin(global_invocation_id) id: vec3<u32>){
	if any(id < vec3(0)) || any(id >= vec3(pc.height, pc.weight, 1)){
		return
	}

	
}

