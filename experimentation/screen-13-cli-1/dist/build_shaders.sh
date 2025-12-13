#! /bin/sh

# run from project root

glslc src/shaders/cube.vert -o dist/shaders/cube.vert.spv
glslc src/shaders/cube.frag -o dist/shaders/cube.frag.spv
