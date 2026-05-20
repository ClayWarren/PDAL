/******************************************************************************
 * Copyright (c) 2015, Peter J. Gadomski <pete.gadomski@gmail.com>
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

#include "OptechReader.hpp"

#include <cmath>
#include <cstring>
#include <limits>

#include <pdal/PDALUtils.hpp>
#include <pdal/SpatialReference.hpp>
#include <pdal/StageFactory.hpp>
#include <pdal/util/IStream.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{
    "readers.optech",
    "Optech reader support.",
    "https://pdal.org/stages/readers.optech.html",
    {"csd"}};

CREATE_STATIC_STAGE(OptechReader, s_info)

std::string OptechReader::getName() const
{
    return s_info.name;
}

#ifndef _MSC_VER
const size_t OptechReader::MaximumNumberOfReturns;
const size_t OptechReader::MaxNumRecordsInBuffer;
const size_t OptechReader::NumBytesInRecord;
#endif

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

OptechReader::OptechReader() : Reader(), m_header() {}

OptechReader::~OptechReader()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

const CsdHeader& OptechReader::getHeader() const
{
    return m_header;
}

void OptechReader::initialize()
{
    std::istream* rawStream = Utils::openFile(m_filename);
    if (!rawStream)
        throwError("Unable to open " + m_filename + " for reading.");
    ILeStream stream(rawStream);

    try
    {
        stream.get(m_header.signature, 4);
        if (strcmp(m_header.signature, "CSD") != 0)
            throwError("Invalid header signature when reading CSD file: '" +
                       std::string(m_header.signature) + "'");

        stream.get(m_header.vendorId, 64);
        stream.get(m_header.softwareVersion, 32);
        stream >> m_header.formatVersion >> m_header.headerSize >>
            m_header.gpsWeek >> m_header.minTime >> m_header.maxTime >>
            m_header.numRecords >> m_header.numStrips;
        for (size_t i = 0; i < 256; ++i)
        {
            stream >> m_header.stripPointers[i];
        }
        stream >> m_header.misalignmentAngles[0] >>
            m_header.misalignmentAngles[1] >> m_header.misalignmentAngles[2] >>
            m_header.imuOffsets[0] >> m_header.imuOffsets[1] >>
            m_header.imuOffsets[2] >> m_header.temperature >> m_header.pressure;
        stream.get(m_header.freeSpace, 830);
    }
    catch (...)
    {
        Utils::closeFile(rawStream);
        throw;
    }
    Utils::closeFile(rawStream);

    setSpatialReference("EPSG:4326");
}

void OptechReader::addDimensions(PointLayoutPtr layout)
{
    using namespace Dimension;

    m_dims = {Id::X,
              Id::Y,
              Id::Z,
              Id::GpsTime,
              Id::ReturnNumber,
              Id::NumberOfReturns,
              Id::EchoRange,
              Id::Intensity,
              Id::ScanAngleRank};
    layout->registerDims(m_dims);
}

void OptechReader::ready(PointTableRef)
{
    m_rustIndex = 0;
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_filename);

    pdal_reader_t* reader = pdal_reader_create_optech(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust Optech reader.");
    }

    m_rustView = pdal_reader_read_first(reader);
    pdal_reader_destroy(reader);
    pdal_options_destroy(options);
    if (!m_rustView)
        throwLastRustError("Rust Optech reader failed.");
}

point_count_t OptechReader::read(PointViewPtr data,
                                 point_count_t countRequested)
{
    point_count_t numRead = 0;
    point_count_t dataIndex = data->size();

    while (numRead < countRequested &&
           m_rustIndex < pdal_point_view_length(m_rustView))
    {
        copyPoint(data, dataIndex);

        if (m_cb)
            m_cb(*data, dataIndex);

        ++dataIndex;
        ++numRead;
        ++m_rustIndex;
    }
    return numRead;
}

void OptechReader::copyPoint(PointViewPtr data, PointId outIdx)
{
    for (Dimension::Id dim : m_dims)
    {
        double value = pdal_point_view_get_f64(m_rustView, m_rustIndex,
                                               Dimension::name(dim).c_str());
        if (dim == Dimension::Id::ScanAngleRank)
            value = std::nextafterf(static_cast<float>(value),
                                    -std::numeric_limits<float>::infinity());
        data->setField(dim, outIdx, value);
    }
}

void OptechReader::done(PointTableRef) {}

} // namespace pdal
