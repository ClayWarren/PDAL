/******************************************************************************
 * Copyright (c) 2015, Howard Butler <howard@hobu.co>
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

#include "GDALReader.hpp"

#include <sstream>

#include <pdal/PointView.hpp>
#include <pdal/private/gdal/Raster.hpp>
#include <pdal/util/Utils.hpp>

namespace pdal
{

static StaticPluginInfo const s_info{"readers.gdal",
                                     "Read GDAL rasters as point clouds.",
                                     "https://pdal.org/stages/reader.gdal.html",
                                     {"tif", "tiff", "jpeg", "jpg", "png"}

};

CREATE_STATIC_STAGE(GDALReader, s_info)

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

std::string GDALReader::getName() const
{
    return s_info.name;
}

GDALReader::GDALReader() : m_blockReader(*this) {}

GDALReader::~GDALReader()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
    m_raster.reset();
}

void GDALReader::initialize()
{
    m_raster.reset(new gdal::Raster(m_filename));
    if (m_raster->open() == gdal::GDALError::CantOpen)
        throwError("Couldn't open raster file '" + m_filename + "'.");

    m_raster->open();
    setSpatialReference(m_raster->getSpatialRef());

    m_width = m_raster->width();
    m_height = m_raster->height();
    m_bandTypes = m_raster->getPDALDimensionTypes();
    m_metadata.add(m_raster->getMetadata());
    m_blockReader.initialize();

    m_dimNames.clear();
    if (m_header.size())
    {
        m_dimNames = Utils::split(m_header, ',');
        if (m_dimNames.size() != m_bandTypes.size())
            throwError("Dimension names are not the same count as "
                       "raster bands.");
    }
    else
    {
        for (size_t i = 0; i < m_bandTypes.size(); ++i)
            m_dimNames.push_back("band_" + std::to_string(i + 1));
    }

    int zBand = 1;
    for (size_t i = 0; i < m_bandIds.size(); ++i)
        if (m_bandIds[i] == Dimension::Id::Z)
        {
            zBand = i + 1;
            break;
        }

    // Bounds is only used in inspect.  We calculate it here so that
    // the raster can be released.
    m_bounds = m_raster->bounds(zBand);

    m_raster.reset();
}

QuickInfo GDALReader::inspect()
{
    QuickInfo qi;

    initialize();

    qi.m_pointCount = m_width * m_height;
    qi.m_srs = getSpatialReference();
    qi.m_bounds = m_bounds;
    qi.m_valid = true;
    qi.m_dimNames = m_dimNames;

    return qi;
}

void GDALReader::addDimensions(PointLayoutPtr layout)
{
    layout->registerDim(pdal::Dimension::Id::X);
    layout->registerDim(pdal::Dimension::Id::Y);

    for (size_t i = 0; i < m_bandTypes.size(); ++i)
    {
        const std::string& name = m_dimNames[i];
        Dimension::Type type = m_bandTypes[i];
        m_bandIds.push_back(layout->registerOrAssignDim(name, type));
    }
}

void GDALReader::addArgs(ProgramArgs& args)
{
    args.add("header",
             "A comma-separated list of dimension IDs to map "
             "raster bands to dimension id",
             m_header);
    args.add("memorycopy",
             "Load the given raster file "
             "entirely to memory",
             m_useMemoryCopy, false)
        .setHidden();
    args.add("gdalopts", "GDAL driver options (name=value,name=value...)",
             m_options);
}

void GDALReader::ready(PointTableRef table)
{
    m_rustIndex = 0;
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_filename);
    addOption(options, "header", m_header);

    pdal_reader_t* reader = pdal_reader_create_gdal(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust GDAL reader.");
    }

    m_rustView = pdal_reader_read_first(reader);
    pdal_reader_destroy(reader);
    pdal_options_destroy(options);
    if (!m_rustView)
        throwLastRustError("Rust GDAL reader failed.");
}

point_count_t GDALReader::read(PointViewPtr view, point_count_t numPts)
{
    point_count_t cnt = 0;
    PointId nextId = view->size();
    while (cnt < numPts && m_rustIndex < pdal_point_view_length(m_rustView))
    {
        PointRef point(*view, nextId);
        processOne(point);
        ++nextId;
        ++cnt;
    }
    return cnt;
}

bool GDALReader::processOne(PointRef& point)
{
    if (m_rustIndex >= pdal_point_view_length(m_rustView))
        return false;

    point.setField(Dimension::Id::X,
                   pdal_point_view_get_f64(m_rustView, m_rustIndex, "X"));
    point.setField(Dimension::Id::Y,
                   pdal_point_view_get_f64(m_rustView, m_rustIndex, "Y"));
    for (size_t band = 0; band < m_bandIds.size(); ++band)
    {
        Dimension::Id id = m_bandIds[band];
        point.setField(id, pdal_point_view_get_f64(m_rustView, m_rustIndex,
                                                   m_dimNames[band].c_str()));
    }
    ++m_rustIndex;
    return true;
}

void GDALReader::done(PointTableRef table)
{
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }
}

GDALReader::BlockReader::BlockReader(GDALReader& reader) : m_reader(reader) {}

void GDALReader::BlockReader::initialize()
{
    m_blockCol = 0;
    m_blockRow = 0;
    m_reader.m_raster->getBlockSize(0, m_blockWidth, m_blockHeight);
    m_numBlocksX = (m_reader.m_width + m_blockWidth - 1) / m_blockWidth;
    m_numBlocksY = (m_reader.m_height + m_blockHeight - 1) / m_blockHeight;
    m_needsRead = true;
    m_colInBlock = 0;
    m_rowInBlock = 0;
}

bool GDALReader::BlockReader::readBlock()
{
    m_needsRead = false;
    if (m_blockRow >= m_numBlocksY)
    {
        return false; // done
    }

    m_currentBlock.m_blockCol = m_blockCol;
    m_currentBlock.m_blockRow = m_blockRow;
    m_currentBlock.m_data.resize(m_reader.m_raster->bandCount());

    int readCol = m_blockCol * m_blockWidth;
    int readRow = m_blockRow * m_blockHeight;

    for (int band = 0; band < m_reader.m_raster->bandCount(); ++band)
    {
        if (m_reader.m_raster->read(
                band, readCol, readRow, m_blockWidth, m_blockHeight,
                m_currentBlock.m_data.at(band)) != gdal::GDALError::None)
        {
            return false;
        }
    }

    m_blockCol++;
    if (m_blockCol >= m_numBlocksX)
    {
        m_blockCol = 0;
        m_blockRow++;
    }

    return true;
}

point_count_t GDALReader::BlockReader::processBlock(PointViewPtr view)
{
    if (!readBlock())
    {
        return 0;
    }

    point_count_t cnt = 0;

    int readCol = m_currentBlock.m_blockCol * m_blockWidth;
    int readRow = m_currentBlock.m_blockRow * m_blockHeight;

    for (int rowInBlock = 0; rowInBlock < m_blockHeight; ++rowInBlock)
    {
        int row = rowInBlock + readRow;
        // We need to check for invalid indices because block sizes don't
        // have to divide the raster size evenly
        if (row >= m_reader.m_height)
            break;

        int rowOffset = rowInBlock * m_blockWidth;
        for (int colInBlock = 0; colInBlock < m_blockWidth; ++colInBlock)
        {
            int col = colInBlock + readCol;
            // We need to check for invalid indices because block sizes don't
            // have to divide the raster size evenly
            if (col >= m_reader.m_width)
                break;

            PointRef point = view->point(view->size());
            m_reader.m_raster->pixelToCoord(col, row, m_coords);
            point.setField(Dimension::Id::X, m_coords[0]);
            point.setField(Dimension::Id::Y, m_coords[1]);
            for (size_t band = 0; band < m_currentBlock.m_data.size(); ++band)
            {
                Dimension::Id id = m_reader.m_bandIds[band];
                point.setField(id, m_currentBlock.m_data.at(band).at(
                                       rowOffset + colInBlock));
            }
            cnt++;
        }
    }

    return cnt;
}

bool GDALReader::BlockReader::processOne(PointRef& point)
{
    if (m_needsRead)
    {
        if (!readBlock())
        {
            return false; // done
        }
    }

    int sample = m_currentBlock.m_blockCol * m_blockWidth + m_colInBlock;
    int line = m_currentBlock.m_blockRow * m_blockHeight + m_rowInBlock;

    m_reader.m_raster->pixelToCoord(sample, line, m_coords);
    point.setField(Dimension::Id::X, m_coords[0]);
    point.setField(Dimension::Id::Y, m_coords[1]);
    for (size_t band = 0; band < m_currentBlock.m_data.size(); ++band)
    {
        Dimension::Id id = m_reader.m_bandIds[band];
        point.setField(id, m_currentBlock.m_data.at(band).at(
                               (m_rowInBlock * m_blockWidth) + m_colInBlock));
    }

    m_colInBlock++;
    // Need to check if col in block or col in raster is out of bounds
    if (m_colInBlock >= m_blockWidth || sample + 1 >= m_reader.m_width)
    {
        m_colInBlock = 0;
        m_rowInBlock++;
        // Need to check if row in block or row in raster is out of bounds
        if (m_rowInBlock >= m_blockHeight || line + 1 >= m_reader.m_height)
        {
            // end of block, need to read a new block
            m_rowInBlock = 0;
            m_needsRead = true;
        }
    }

    return true;
}

} // namespace pdal
