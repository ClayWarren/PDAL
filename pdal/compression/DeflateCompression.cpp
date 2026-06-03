/******************************************************************************
 * Copyright (c) 2014, Howard Butler (howard@hobu.co)
 *
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following
 * conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright
 *       notice, this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above copyright
 *       notice, this list of conditions and the following disclaimer in
 *       the documentation and/or other materials provided
 *       with the distribution.
 *     * Neither the name of Hobu, Inc. or Flaxen Geo Consulting nor the
 *       names of its contributors may be used to endorse or promote
 *       products derived from this software without specific prior
 *       written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 * FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 * COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS
 * OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED
 * AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT
 * OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 ****************************************************************************/

#include "DeflateCompression.hpp"
#include "GzipCompression.hpp"

#include <pdal_capi.h>

namespace pdal
{

namespace
{

compression_error lastCompressionError(const std::string& fallback)
{
    const char* message = pdal_last_error();
    return compression_error(message && message[0] ? message : fallback);
}

// Forward Rust-produced output bytes to the block callback and release them.
void emitRustBytes(const BlockCb& cb, uint8_t* buf, size_t len)
{
    if (!buf)
        return;
    cb(reinterpret_cast<char*>(buf), len);
    pdal_u8_array_free(buf, len);
}

} // unnamed namespace

// The deflate compressor routes through the Rust C ABI, which produces the
// same zlib wire format as zlib's deflateInit default.
class DeflateCompressorImpl
{
public:
    DeflateCompressorImpl(BlockCb cb) : m_cb(cb)
    {
        m_compressor = pdal_deflate_compressor_create();
        if (!m_compressor)
            throw compression_error("Could not create deflate compressor.");
    }

    ~DeflateCompressorImpl()
    {
        if (m_compressor)
            pdal_deflate_compressor_destroy(m_compressor);
    }

    void compress(const char* buf, size_t bufsize)
    {
        uint8_t* out = nullptr;
        size_t outlen = 0;
        if (!pdal_deflate_compressor_update(m_compressor, buf, bufsize, &out,
                                            &outlen))
            throw lastCompressionError(
                "Rust deflate compressor update failed.");
        emitRustBytes(m_cb, out, outlen);
    }

    void done()
    {
        uint8_t* out = nullptr;
        size_t outlen = 0;
        if (!pdal_deflate_compressor_finish(m_compressor, &out, &outlen))
            throw lastCompressionError(
                "Rust deflate compressor finish failed.");
        emitRustBytes(m_cb, out, outlen);
    }

private:
    BlockCb m_cb;
    pdal_deflate_compressor_t* m_compressor = nullptr;
};

DeflateCompressor::DeflateCompressor(BlockCb cb)
    : m_impl(new DeflateCompressorImpl(cb))
{
}

DeflateCompressor::~DeflateCompressor() {}

void DeflateCompressor::compress(const char* buf, size_t bufsize)
{
    m_impl->compress(buf, bufsize);
}

void DeflateCompressor::done()
{
    m_impl->done();
}

// The plain deflate and gzip auto-detect decompressors route through the Rust C
// ABI. `windowBits == 47` mirrors zlib's gzip-or-zlib auto-detect mode.
class DeflateDecompressorImpl
{
public:
    DeflateDecompressorImpl(BlockCb cb, int windowBits = 15) : m_cb(cb)
    {
        if (windowBits == 15)
            m_decompressor = pdal_deflate_decompressor_create();
        else if (windowBits == 47)
            m_decompressor = pdal_deflate_auto_decompressor_create();
        else
        {
            throw compression_error("Unsupported deflate window bits.");
        }
        if (!m_decompressor)
            throw compression_error("Could not create deflate decompressor.");
    }

    ~DeflateDecompressorImpl()
    {
        if (m_decompressor)
            pdal_deflate_decompressor_destroy(m_decompressor);
    }

    void decompress(const char* buf, size_t bufsize)
    {
        uint8_t* out = nullptr;
        size_t outlen = 0;
        if (!pdal_deflate_decompressor_update(m_decompressor, buf, bufsize,
                                              &out, &outlen))
            throw lastCompressionError(
                "Rust deflate decompressor update failed.");
        emitRustBytes(m_cb, out, outlen);
    }

    void done()
    {
        uint8_t* out = nullptr;
        size_t outlen = 0;
        if (!pdal_deflate_decompressor_finish(m_decompressor, &out, &outlen))
            throw lastCompressionError(
                "Rust deflate decompressor finish failed.");
        emitRustBytes(m_cb, out, outlen);
    }

private:
    BlockCb m_cb;
    pdal_deflate_decompressor_t* m_decompressor = nullptr;
};

DeflateDecompressor::DeflateDecompressor(BlockCb cb)
    : m_impl(new DeflateDecompressorImpl(cb))
{
}

DeflateDecompressor::~DeflateDecompressor() {}

void DeflateDecompressor::decompress(const char* buf, size_t bufsize)
{
    m_impl->decompress(buf, bufsize);
}

void DeflateDecompressor::done()
{
    m_impl->done();
}

// GZIP

GzipDecompressor::GzipDecompressor(BlockCb cb)
    : m_impl(new DeflateDecompressorImpl(cb, 47))
{
}

GzipDecompressor::~GzipDecompressor() {}

void GzipDecompressor::decompress(const char* buf, size_t bufsize)
{
    m_impl->decompress(buf, bufsize);
}

void GzipDecompressor::done()
{
    m_impl->done();
}

} // namespace pdal
