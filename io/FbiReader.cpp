/******************************************************************************
 * Copyright (c) 2021, Antoine Lavenant, antoine.lavenant@ign.fr
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

#include "FbiReader.hpp"

#include <pdal/PointView.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{"readers.fbi",
                                     "Fbi Reader",
                                     "https://pdal.org/stages/readers.fbi.html",
                                     {"bin", "fbi"}};

CREATE_STATIC_STAGE(FbiReader, s_info)

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

Dimension::Type rustDimType(int typeId)
{
    switch (typeId)
    {
    case 0:
        return Dimension::Type::Unsigned8;
    case 1:
        return Dimension::Type::Unsigned16;
    case 2:
        return Dimension::Type::Unsigned32;
    case 3:
        return Dimension::Type::Unsigned64;
    case 4:
        return Dimension::Type::Signed8;
    case 5:
        return Dimension::Type::Signed16;
    case 6:
        return Dimension::Type::Signed32;
    case 7:
        return Dimension::Type::Signed64;
    case 8:
        return Dimension::Type::Float;
    case 9:
        return Dimension::Type::Double;
    default:
        return Dimension::Type::Double;
    }
}

} // namespace

FbiReader::FbiReader() : pdal::Reader() {}

FbiReader::~FbiReader()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

std::string FbiReader::getName() const
{
    return s_info.name;
}

void FbiReader::initialize()
{
    pdal_fbi_header_info_t info;
    if (pdal_fbi_header_info(m_filename.c_str(), &info) != 0)
        throwLastRustError("Rust FBI reader failed to inspect header.");

    m_header.Version = info.version;
    m_header.HdrSize = info.header_size;
    m_header.FastCnt = info.point_count;
    m_header.PosXyz = info.xyz_position;

    loadRustView();
}

void FbiReader::addArgs(ProgramArgs& args)
{
    // nothing for now
}

void FbiReader::addDimensions(PointLayoutPtr layout)
{
    m_dims.clear();
    uint64_t dimCount = pdal_point_view_dim_count(m_rustView);
    for (uint64_t idx = 0; idx < dimCount; ++idx)
    {
        char* rawName = pdal_point_view_dim_name(m_rustView, idx);
        if (!rawName)
            continue;
        std::string name(rawName);
        pdal_string_free(rawName);
        Dimension::Id id = layout->registerOrAssignDim(
            name, rustDimType(pdal_point_view_dim_type(m_rustView, idx)));
        if (id != Dimension::Id::Unknown)
            m_dims.push_back(id);
    }
}

void FbiReader::ready(PointTableRef)
{
    m_rustIndex = 0;
    if (!m_rustView)
        loadRustView();
}

void FbiReader::loadRustView()
{
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_filename);

    pdal_reader_t* reader = pdal_reader_create_fbi(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust FBI reader.");
    }

    m_rustView = pdal_reader_read_first(reader);
    pdal_reader_destroy(reader);
    pdal_options_destroy(options);
    if (!m_rustView)
        throwLastRustError("Rust FBI reader failed.");
}

point_count_t FbiReader::read(PointViewPtr view, point_count_t count)
{
    point_count_t numRead = 0;
    PointId nextId = view->size();
    while (numRead < count && m_rustIndex < pdal_point_view_length(m_rustView))
    {
        copyPoint(view, nextId);
        if (m_cb)
            m_cb(*view, nextId);

        nextId++;
        m_rustIndex++;
        numRead++;
    }

    return numRead;
}

void FbiReader::copyPoint(PointViewPtr view, PointId outIdx)
{
    for (Dimension::Id dim : m_dims)
    {
        view->setField(dim, outIdx,
                       pdal_point_view_get_f64(m_rustView, m_rustIndex,
                                               Dimension::name(dim).c_str()));
    }
}

void FbiReader::done(PointTableRef) {}

} // namespace pdal
