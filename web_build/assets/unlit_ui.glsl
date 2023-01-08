#VERTEX 

#INCLUDE standard_vertex

#FRAGMENT

in vec2 f_texture_coordinates;
in vec4 f_vertex_color;

out vec4 color_out;

uniform sampler2D sp0_base_color_texture;

void main()
{
    if (f_texture_coordinates.x != 0.0 || f_texture_coordinates.y != 0.0) {
        float alpha = texture(sp0_base_color_texture, f_texture_coordinates).r;
        color_out = vec4(f_vertex_color.rgb * alpha, alpha);
    } else {
        color_out = f_vertex_color;
    }
    //color_out.rgb = pow(color_out.rgb, vec3(1.0/2.2));
    //color_out = vec4(1.0);
}