@echo off
rem run from project root

D:\Programs\vcpkg\installed\x64-windows\tools\shaderc\glslc src\shaders\cube.vert -o dist\shaders\cube.vert.spv
D:\Programs\vcpkg\installed\x64-windows\tools\shaderc\glslc src\shaders\cube.frag -o dist\shaders\cube.frag.spv
