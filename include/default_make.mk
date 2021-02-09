

#Automatic dependency generation: Makes GCC generate the dependencies needed for a cpp file
#excluding system header files.
CPPFLAGS+=-MMD \
	  -MP

