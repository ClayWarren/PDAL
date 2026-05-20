/******************************************************************************
 * Copyright (c) 2014, Peter J. Gadomski (pete.gadomski@gmail.com)
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

#include "SbetSmrmsgReader.hpp"
#include "SbetCommon.hpp"

#include <pdal/PointRef.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{
    "readers.smrmsg",
    "SBET smrmsg Reader",
    "https://pdal.org/stages/readers.smrmsg.html",
    {"smrmsg"}};

CREATE_STATIC_STAGE(SmrmsgReader, s_info)

namespace
{

void addOption(pdal_options_t* options, const std::string& key,
               const std::string& value)
{
    pdal_options_add_str(options, key.c_str(), value.c_str());
}

void throwLastRustError(const std::string& fallback)
{
    const char* message = pdal_last_error();
    if (message && message[0])
        throw pdal_error(message);
    throw pdal_error(fallback);
}

} // namespace

SmrmsgReader::~SmrmsgReader()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

std::string SmrmsgReader::getName() const
{
    return s_info.name;
}

void SmrmsgReader::addArgs(ProgramArgs& args) {}

void SmrmsgReader::addDimensions(PointLayoutPtr layout)
{
    layout->registerDims(sbet::smrmsgFileDimensions());
}

void SmrmsgReader::ready(PointTableRef)
{
    m_rustIndex = 0;
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_filename);

    pdal_reader_t* reader = pdal_reader_create_smrmsg(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust SMRMSG reader.");
    }

    m_rustView = pdal_reader_read_first(reader);
    pdal_reader_destroy(reader);
    pdal_options_destroy(options);
    if (!m_rustView)
        throwLastRustError("Rust SMRMSG reader failed.");

    m_dims = sbet::smrmsgFileDimensions();
}

bool SmrmsgReader::processOne(PointRef& point)
{
    if (m_rustIndex >= pdal_point_view_length(m_rustView))
        return false;

    for (auto di = m_dims.begin(); di != m_dims.end(); ++di)
    {
        Dimension::Id dim = *di;
        point.setField(dim,
                       pdal_point_view_get_f64(m_rustView, m_rustIndex,
                                               Dimension::name(dim).c_str()));
    }
    m_rustIndex++;
    return true;
}

point_count_t SmrmsgReader::read(PointViewPtr view, point_count_t count)
{
    PointId nextId = view->size();
    point_count_t numRead = 0;
    while (numRead < count && m_rustIndex < pdal_point_view_length(m_rustView))
    {
        PointRef point = view->point(nextId);
        processOne(point);
        if (m_cb)
            m_cb(*view, nextId);

        nextId++;
        numRead++;
    }
    return numRead;
}

bool SmrmsgReader::eof()
{
    return m_rustIndex >= pdal_point_view_length(m_rustView);
}

} // namespace pdal
