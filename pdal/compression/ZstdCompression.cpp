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

#include "ZstdCompression.hpp"

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

void emitRustBytes(const BlockCb& cb, uint8_t* buf, size_t len)
{
    if (!buf)
        return;
    cb(reinterpret_cast<char*>(buf), len);
    pdal_u8_array_free(buf, len);
}

} // unnamed namespace

class ZstdCompressorImpl
{
public:
    ZstdCompressorImpl(BlockCb cb, int compressionLevel) : m_cb(cb)
    {
        m_compressor = pdal_zstd_compressor_create(compressionLevel);
        if (!m_compressor)
            throw lastCompressionError("Could not create zstd compressor.");
    }

    ~ZstdCompressorImpl()
    {
        if (m_compressor)
            pdal_zstd_compressor_destroy(m_compressor);
    }

    void compress(const char* buf, size_t bufsize)
    {
        uint8_t* out = nullptr;
        size_t outlen = 0;
        if (!pdal_zstd_compressor_update(m_compressor, buf, bufsize, &out,
                                         &outlen))
            throw lastCompressionError("Rust zstd compressor update failed.");
        emitRustBytes(m_cb, out, outlen);
    }

    void done()
    {
        uint8_t* out = nullptr;
        size_t outlen = 0;
        if (!pdal_zstd_compressor_finish(m_compressor, &out, &outlen))
            throw lastCompressionError("Rust zstd compressor finish failed.");
        emitRustBytes(m_cb, out, outlen);
    }

private:
    BlockCb m_cb;
    pdal_zstd_compressor_t* m_compressor = nullptr;
};

ZstdCompressor::ZstdCompressor(BlockCb cb)
    : m_impl(new ZstdCompressorImpl(cb, 15))
{
}

ZstdCompressor::ZstdCompressor(BlockCb cb, int compressionLevel)
    : m_impl(new ZstdCompressorImpl(cb, compressionLevel))
{
}

ZstdCompressor::~ZstdCompressor() {}

void ZstdCompressor::compress(const char* buf, size_t bufsize)
{
    m_impl->compress(buf, bufsize);
}

void ZstdCompressor::done()
{
    m_impl->done();
}

class ZstdDecompressorImpl
{
public:
    ZstdDecompressorImpl(BlockCb cb) : m_cb(cb)
    {
        m_decompressor = pdal_zstd_decompressor_create();
        if (!m_decompressor)
            throw lastCompressionError("Could not create zstd decompressor.");
    }

    ~ZstdDecompressorImpl()
    {
        if (m_decompressor)
            pdal_zstd_decompressor_destroy(m_decompressor);
    }

    void decompress(const char* buf, size_t bufsize)
    {
        uint8_t* out = nullptr;
        size_t outlen = 0;
        if (!pdal_zstd_decompressor_update(m_decompressor, buf, bufsize, &out,
                                           &outlen))
            throw lastCompressionError("Rust zstd decompressor update failed.");
        emitRustBytes(m_cb, out, outlen);
    }

private:
    BlockCb m_cb;
    pdal_zstd_decompressor_t* m_decompressor = nullptr;
};

ZstdDecompressor::ZstdDecompressor(BlockCb cb)
    : m_impl(new ZstdDecompressorImpl(cb))
{
}

ZstdDecompressor::~ZstdDecompressor() {}

void ZstdDecompressor::decompress(const char* buf, size_t bufsize)
{
    m_impl->decompress(buf, bufsize);
}

} // namespace pdal
