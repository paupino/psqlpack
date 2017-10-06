SHELL := /bin/bash

OSFLAG 				:=
ifeq ($(OS),Windows_NT)
	OSFLAG += WIN32
else
	UNAME_S := $(shell uname -s)
	ifeq ($(UNAME_S),Linux)
		OSFLAG += LINUX
	endif
	ifeq ($(UNAME_S),Darwin)
		OSFLAG += OSX
	endif
endif

RM_BK 		:=
ifeq ($(OS),Windows_NT)
	RM_BK += del /S *.bk
else
	RM_BK += find . -name *.bk -delete
endif

clean:
	$(RM_BK) 

build:
	cargo build -p psqlpack-cli