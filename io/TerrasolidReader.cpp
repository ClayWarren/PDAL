/******************************************************************************
 * Copyright (c) 2011, Howard Butler, hobu.inc@gmail.com
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

#include "TerrasolidReader.hpp"

#include <pdal/PointView.hpp>
#include <pdal/util/IStream.hpp>

#include <algorithm>

namespace pdal
{

static StaticPluginInfo const s_info{
    "readers.terrasolid",
    "TerraSolid Reader",
    "https://pdal.org/stages/readers.terrasolid.html",
    {"bin"}};

CREATE_STATIC_STAGE(TerrasolidReader, s_info)

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

std::string TerrasolidReader::getName() const
{
    return s_info.name;
}

TerrasolidReader::~TerrasolidReader()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

void TerrasolidReader::initialize()
{
    ILeStream stream(m_filename);

    TerraSolidHeaderPtr h(new TerraSolidHeader);
    m_header.swap(h);

    stream >> m_header->HdrSize >> m_header->HdrVersion >> m_header->RecogVal;
    stream.get(m_header->RecogStr, 4);
    stream >> m_header->PntCnt >> m_header->Units >> m_header->OrgX >>
        m_header->OrgY >> m_header->OrgZ >> m_header->Time >> m_header->Color;

    if (m_header->RecogVal != 970401)
        throwError("Header identifier was not '970401', is this "
                   "a TerraSolid .bin file?");

    m_haveColor = (m_header->Color != 0);
    m_haveTime = (m_header->Time != 0);
    m_format = static_cast<TERRASOLID_Format_Type>(m_header->HdrVersion);

    if ((m_format != TERRASOLID_Format_1) && (m_format != TERRASOLID_Format_2))
        throwError("Version was '" + Utils::toString(m_format) + "', not '" +
                   Utils::toString(TERRASOLID_Format_1) + "' or '" +
                   Utils::toString(TERRASOLID_Format_2) + "'");

    log()->get(LogLevel::Debug)
        << "TerraSolid Reader::initialize format: " << m_format << '\n';
    log()->get(LogLevel::Debug) << "OrgX: " << m_header->OrgX << '\n';
    log()->get(LogLevel::Debug) << "OrgY: " << m_header->OrgY << '\n';
    log()->get(LogLevel::Debug) << "OrgZ: " << m_header->OrgZ << '\n';
    log()->get(LogLevel::Debug) << "Units: " << m_header->Units << '\n';
    log()->get(LogLevel::Debug) << "Time: " << m_header->Time << '\n';
    log()->get(LogLevel::Debug) << "Color: " << m_header->Color << '\n';
    log()->get(LogLevel::Debug) << "Count: " << m_header->PntCnt << '\n';
    log()->get(LogLevel::Debug) << "RecogVal: " << m_header->RecogVal << '\n';
}

void TerrasolidReader::addDimensions(PointLayoutPtr layout)
{
    m_dims = {Dimension::Id::Classification,
              Dimension::Id::PointSourceId,
              Dimension::Id::Intensity,
              Dimension::Id::X,
              Dimension::Id::Y,
              Dimension::Id::Z,
              Dimension::Id::ReturnNumber,
              Dimension::Id::NumberOfReturns};
    if (m_format == TERRASOLID_Format_2)
    {
        m_dims.push_back(Dimension::Id::Flag);
        m_dims.push_back(Dimension::Id::Mark);
    }

    if (m_haveTime)
        m_dims.push_back(Dimension::Id::OffsetTime);

    if (m_haveColor)
    {
        m_dims.push_back(Dimension::Id::Red);
        m_dims.push_back(Dimension::Id::Green);
        m_dims.push_back(Dimension::Id::Blue);
        m_dims.push_back(Dimension::Id::Alpha);
    }

    for (Dimension::Id dim : m_dims)
        layout->registerDim(dim);
}

void TerrasolidReader::ready(PointTableRef)
{
    m_rustIndex = 0;
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_filename);

    pdal_reader_t* reader = pdal_reader_create_terrasolid(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust TerraSolid reader.");
    }

    m_rustView = pdal_reader_read_first(reader);
    pdal_reader_destroy(reader);
    pdal_options_destroy(options);
    if (!m_rustView)
        throwLastRustError("Rust TerraSolid reader failed.");
}

point_count_t TerrasolidReader::read(PointViewPtr view, point_count_t count)
{
    count = (std::min)(count, getNumPoints() - m_rustIndex);

    PointId nextId = view->size();
    point_count_t numRead = 0;
    while (numRead < count && !eof())
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

void TerrasolidReader::copyPoint(PointViewPtr view, PointId outIdx)
{
    for (Dimension::Id dim : m_dims)
    {
        view->setField(dim, outIdx,
                       pdal_point_view_get_f64(m_rustView, m_rustIndex,
                                               Dimension::name(dim).c_str()));
    }
}

void TerrasolidReader::done(PointTableRef) {}

} // namespace pdal
