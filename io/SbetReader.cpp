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

#include "SbetReader.hpp"
#include "SbetCommon.hpp"

#include "private/connector/Connector.hpp"

#include <pdal/PointRef.hpp>
#include <pdal/util/FileUtils.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

namespace pdal
{

struct SbetReader::Private
{
    pdal_point_view_t* rustView = nullptr;
    PointId rustIndex = 0;
    Dimension::IdList dims;
    bool anglesAsDegrees;

    std::unique_ptr<connector::Connector> connector;
    bool isRemote = false;
};

SbetReader::SbetReader() : m_private(new Private) {}

SbetReader::~SbetReader()
{
    if (m_private->rustView)
        pdal_point_view_destroy(m_private->rustView);
    cleanup();
}

static StaticPluginInfo const s_info{
    "readers.sbet",
    "SBET Reader",
    "https://pdal.org/stages/readers.sbet.html",
    {"sbet"}};

CREATE_STATIC_STAGE(SbetReader, s_info)

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

std::string SbetReader::getName() const
{
    return s_info.name;
}

void SbetReader::addArgs(ProgramArgs& args)
{
    args.add("angles_as_degrees", "Convert all angles to degrees",
             m_private->anglesAsDegrees, true);
}

void SbetReader::addDimensions(PointLayoutPtr layout)
{
    layout->registerDims(sbet::fileDimensions());
}

void SbetReader::ready(PointTableRef)
{
    tryLoadRemote();

    m_private->rustIndex = 0;
    if (m_private->rustView)
    {
        pdal_point_view_destroy(m_private->rustView);
        m_private->rustView = nullptr;
    }

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_filename);
    addOption(options, "angles_as_degrees",
              m_private->anglesAsDegrees ? "true" : "false");

    pdal_reader_t* reader = pdal_reader_create_sbet(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust SBET reader.");
    }

    m_private->rustView = pdal_reader_read_first(reader);
    pdal_reader_destroy(reader);
    pdal_options_destroy(options);
    if (!m_private->rustView)
        throwLastRustError("Rust SBET reader failed.");

    m_private->dims = sbet::fileDimensions();
}

bool SbetReader::processOne(PointRef& point)
{
    if (m_private->rustIndex >= pdal_point_view_length(m_private->rustView))
        return false;

    for (auto di = m_private->dims.begin(); di != m_private->dims.end(); ++di)
    {
        Dimension::Id dim = *di;
        point.setField(dim, pdal_point_view_get_f64(
                                m_private->rustView, m_private->rustIndex,
                                Dimension::name(dim).c_str()));
    }
    m_private->rustIndex++;
    return true;
}

point_count_t SbetReader::read(PointViewPtr view, point_count_t count)
{
    PointId nextId = view->size();
    point_count_t numRead = 0;
    while (numRead < count &&
           m_private->rustIndex < pdal_point_view_length(m_private->rustView))
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

bool SbetReader::eof()
{
    return m_private->rustIndex >= pdal_point_view_length(m_private->rustView);
}

void SbetReader::done(PointTableRef table)
{
    cleanup();
}

void SbetReader::cleanup()
{
    if (m_private->isRemote)
        FileUtils::deleteFile(m_filename);
}

void SbetReader::tryLoadRemote()
{
    m_private->connector.reset(new connector::Connector(m_filespec));
    m_private->isRemote = Utils::isRemote(m_filename);
    auto handle = m_private->connector->getLocalHandle(m_filename);
    m_filename = handle.release();
}

} // namespace pdal
