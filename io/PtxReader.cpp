/******************************************************************************
 * Copyright (c) 2022, Daniel Brookes (dbrookes@micromine.com)
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
 *     * Neither the name of Hobu, Inc. nor the
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

#include "PtxReader.hpp"

namespace pdal
{

static const StaticPluginInfo sc_info{
    "readers.ptx",
    "Ptx Reader",
    "https://pdal.org/stages/readers.ptx.html",
    {"ptx"}};

CREATE_STATIC_STAGE(PtxReader, sc_info);

namespace
{

void addOption(pdal_options_t* options, const std::string& key,
               const std::string& value)
{
    pdal_options_add_str(options, key.c_str(), value.c_str());
}

void addOption(pdal_options_t* options, const std::string& key, bool value)
{
    addOption(options, key, std::string(value ? "true" : "false"));
}

void throwLastRustError(const std::string& fallback)
{
    const char* message = pdal_last_error();
    if (message && message[0])
        throw pdal_error(message);
    throw pdal_error(fallback);
}

} // namespace

PtxReader::~PtxReader()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

std::string PtxReader::getName() const
{
    return sc_info.name;
}

void PtxReader::initialize(PointTableRef table)
{
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }
    m_rustIndex = 0;

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_filename);
    addOption(options, "discard_missing_points", m_discardMissingPoints);

    pdal_reader_t* reader = pdal_reader_create_ptx(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust PTX reader.");
    }

    m_rustView = pdal_reader_read_first(reader);
    pdal_reader_destroy(reader);
    pdal_options_destroy(options);
    if (!m_rustView)
        throwLastRustError("Rust PTX reader failed.");
}

void PtxReader::addArgs(ProgramArgs& args)
{
    args.add("discard_missing_points",
             "Skip over missing input points with XYZ values of \"0 0 0\".",
             m_discardMissingPoints, true);
}

void PtxReader::addDimensions(PointLayoutPtr layout)
{
    m_dimensions.clear();
    uint64_t dimCount = pdal_point_view_dim_count(m_rustView);
    for (uint64_t idx = 0; idx < dimCount; ++idx)
    {
        char* rawName = pdal_point_view_dim_name(m_rustView, idx);
        if (!rawName)
            continue;
        std::string name(rawName);
        pdal_string_free(rawName);
        Dimension::Id id =
            layout->registerOrAssignDim(name, Dimension::Type::Double);
        m_dimensions.push_back(id);
    }
}

void PtxReader::ready(PointTableRef table)
{
    m_rustIndex = 0;
}

point_count_t PtxReader::read(PointViewPtr view, point_count_t numPts)
{
    point_count_t count = 0;
    while (count < numPts && m_rustIndex < pdal_point_view_length(m_rustView))
    {
        PointId outIdx = view->size();
        view->point(outIdx);
        for (Dimension::Id dim : m_dimensions)
        {
            view->setField(
                dim, outIdx,
                pdal_point_view_get_f64(m_rustView, m_rustIndex,
                                        view->layout()->dimName(dim).c_str()));
        }
        count++;
        m_rustIndex++;
    }

    return count;
}

void PtxReader::done(PointTableRef table) {}

} // namespace pdal
