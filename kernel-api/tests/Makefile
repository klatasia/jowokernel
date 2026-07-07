# JowoKernel API Tests
# Copyright (c) 2026 KLAT OS Team
# SPDX-License-Identifier: MIT

CC ?= clang
CFLAGS ?= -Wall -Wextra -g -I..

.PHONY: all test clean

all: test

test: test_kernel_api
	@echo "Running tests..."
	./test_kernel_api

test_kernel_api: tests/test_kernel_api.c ../kernel-api/errors/error_impl.cpp
	$(CC) $(CFLAGS) -o $@ tests/test_kernel_api.c ../kernel-api/errors/error_impl.cpp

clean:
	rm -f test_kernel_api
