#VERTEX 
#INCLUDE standard_vertex
#FRAGMENT

in vec2 f_texture_coordinates;
in vec4 f_vertex_color;

uniform vec2 p_texture_coordinate_offset;
uniform vec2 p_texture_coordinate_scale;

uniform vec4 p_base_color;
uniform sampler2D sp0_base_color_texture;

out vec4 color_out;

void main()
{
    vec4 v = texture(sp0_base_color_texture, f_texture_coordinates);
    float brightness = (v.r + v.g + v.b) / 3.0;

    color_out = vec4((f_vertex_color.rgb * brightness * p_base_color.rgb), f_vertex_color.a * p_base_color.a * v.a) ;
    if (color_out.a == 0.0) {
        discard;
    }
}