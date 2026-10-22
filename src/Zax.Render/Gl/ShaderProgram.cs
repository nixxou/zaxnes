using System.Numerics;
using Silk.NET.OpenGL;

namespace Zax.Render.Gl;

/// <summary>
/// A compiled and linked GLSL program, with a cache of uniform locations.
/// </summary>
/// <remarks>
/// OpenGL 3.3 core without any extension (REQ-SCN-056). The shaders stay close to GLES 3.0 so
/// that they can be reused there (REQ-ARCH-030): no explicit uniform locations, no bindings in
/// the source, no integer texture gathers.
/// </remarks>
internal sealed class ShaderProgram : IDisposable
{
    private readonly GL _gl;
    private readonly Dictionary<string, int> _uniforms = [];
    private uint _handle;

    private ShaderProgram(GL gl, uint handle)
    {
        _gl = gl;
        _handle = handle;
    }

    /// <summary>The program name.</summary>
    public uint Handle => _handle;

    /// <summary>Compiles and links a vertex and a fragment shader.</summary>
    /// <param name="gl">The GL API of the current context.</param>
    /// <param name="name">Name used in error messages.</param>
    /// <param name="vertexSource">Vertex shader source.</param>
    /// <param name="fragmentSource">Fragment shader source.</param>
    /// <exception cref="InvalidOperationException">Compilation or linking failed.</exception>
    public static ShaderProgram Create(GL gl, string name, string vertexSource, string fragmentSource)
    {
        ArgumentNullException.ThrowIfNull(gl);

        uint vertex = Compile(gl, name, ShaderType.VertexShader, vertexSource);
        uint fragment = 0;
        try
        {
            fragment = Compile(gl, name, ShaderType.FragmentShader, fragmentSource);

            uint program = gl.CreateProgram();
            gl.AttachShader(program, vertex);
            gl.AttachShader(program, fragment);
            gl.LinkProgram(program);
            gl.GetProgram(program, ProgramPropertyARB.LinkStatus, out int status);
            if (status == 0)
            {
                string log = gl.GetProgramInfoLog(program);
                gl.DeleteProgram(program);
                throw new InvalidOperationException($"Linking the program '{name}' failed: {log}");
            }

            gl.DetachShader(program, vertex);
            gl.DetachShader(program, fragment);
            return new ShaderProgram(gl, program);
        }
        finally
        {
            gl.DeleteShader(vertex);
            if (fragment != 0)
            {
                gl.DeleteShader(fragment);
            }
        }
    }

    /// <summary>Makes the program current.</summary>
    public void Use() => _gl.UseProgram(_handle);

    /// <summary>Location of <paramref name="name"/>, or −1 when the uniform is not active.</summary>
    public int Location(string name)
    {
        if (_uniforms.TryGetValue(name, out int location))
        {
            return location;
        }

        location = _gl.GetUniformLocation(_handle, name);
        _uniforms[name] = location;
        return location;
    }

    /// <summary>Sets an integer uniform.</summary>
    public void Set(string name, int value)
    {
        int location = Location(name);
        if (location >= 0)
        {
            _gl.Uniform1(location, value);
        }
    }

    /// <summary>Sets a float uniform.</summary>
    public void Set(string name, float value)
    {
        int location = Location(name);
        if (location >= 0)
        {
            _gl.Uniform1(location, value);
        }
    }

    /// <summary>Sets a <c>vec2</c> uniform.</summary>
    public void Set(string name, Vector2 value)
    {
        int location = Location(name);
        if (location >= 0)
        {
            _gl.Uniform2(location, value.X, value.Y);
        }
    }

    /// <summary>Sets a <c>vec3</c> uniform.</summary>
    public void Set(string name, Vector3 value)
    {
        int location = Location(name);
        if (location >= 0)
        {
            _gl.Uniform3(location, value.X, value.Y, value.Z);
        }
    }

    /// <summary>Sets a <c>vec4</c> uniform.</summary>
    public void Set(string name, Vector4 value)
    {
        int location = Location(name);
        if (location >= 0)
        {
            _gl.Uniform4(location, value.X, value.Y, value.Z, value.W);
        }
    }

    /// <summary>
    /// Sets a <c>mat4</c> uniform from a <see cref="Matrix4x4"/>. The matrix is stored row by
    /// row and applied to row vectors, so uploading it untransposed gives GLSL the
    /// column-vector matrix the shaders expect.
    /// </summary>
    public unsafe void Set(string name, Matrix4x4 value)
    {
        int location = Location(name);
        if (location < 0)
        {
            return;
        }

        float* cells = stackalloc float[16]
        {
            value.M11, value.M12, value.M13, value.M14,
            value.M21, value.M22, value.M23, value.M24,
            value.M31, value.M32, value.M33, value.M34,
            value.M41, value.M42, value.M43, value.M44,
        };
        _gl.UniformMatrix4(location, 1, false, cells);
    }

    /// <summary>Sets a <c>mat3</c> uniform from the nine cells, row by row.</summary>
    public unsafe void SetMatrix3(string name, ReadOnlySpan<float> rowMajor)
    {
        int location = Location(name);
        if (location < 0)
        {
            return;
        }

        // GLSL reads a mat3 column by column, so the row-major cells are uploaded transposed.
        float* cells = stackalloc float[9]
        {
            rowMajor[0], rowMajor[3], rowMajor[6],
            rowMajor[1], rowMajor[4], rowMajor[7],
            rowMajor[2], rowMajor[5], rowMajor[8],
        };
        _gl.UniformMatrix3(location, 1, false, cells);
    }

    /// <summary>Sets one element of a <c>vec4</c> array uniform.</summary>
    public void SetArray(string name, int index, Vector4 value)
        => Set($"{name}[{index}]", value);

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_handle == 0)
        {
            return;
        }

        _gl.DeleteProgram(_handle);
        _handle = 0;
    }

    private static uint Compile(GL gl, string name, ShaderType type, string source)
    {
        uint shader = gl.CreateShader(type);
        gl.ShaderSource(shader, source);
        gl.CompileShader(shader);
        gl.GetShader(shader, ShaderParameterName.CompileStatus, out int status);
        if (status != 0)
        {
            return shader;
        }

        string log = gl.GetShaderInfoLog(shader);
        gl.DeleteShader(shader);
        throw new InvalidOperationException($"Compiling the {type} of '{name}' failed: {log}");
    }
}
