/******************************************************************************
 * Copyright (c) 2020, Hobu Inc. <info@hobu.co>
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
 *     * Neither the name of Hobu, Inc. nor the names of its contributors
 *       may be used to endorse or promote products derived from this
 *       software without specific prior written permission.
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

#include "RasterWriter.hpp"
#include <pdal/private/RustViewConverter.hpp>

#include <pdal/PointView.hpp>
#include <pdal_capi.h>

namespace pdal
{

static StaticPluginInfo const s_info{
    "writers.raster",
    "Write a raster.",
    "https://pdal.org/stages/writers.raster.html",
    {}};

CREATE_STATIC_STAGE(RasterWriter, s_info)

std::string RasterWriter::getName() const
{
    return s_info.name;
}

namespace
{

void addOption(pdal_options_t* options, const std::string& key,
               const std::string& value)
{
    pdal_options_add_str(options, key.c_str(), value.c_str());
}

void addOption(pdal_options_t* options, const std::string& key, double value)
{
    pdal_options_add_f64(options, key.c_str(), value);
}

} // unnamed namespace

RasterWriter::RasterWriter() {}

RasterWriter::~RasterWriter()
{
    for (pdal_point_view_t* view : m_rustViews)
        pdal_point_view_destroy(view);
}

void RasterWriter::addArgs(ProgramArgs& args)
{
    args.add("gdaldriver", "GDAL driver name", m_drivername, "GTiff");
    args.add("gdalopts", "GDAL driver options (name=value,name=value...)",
             m_options);
    args.add("rasters", "List of raster names to write as bands.",
             m_rasterNames);
    args.add("data_type",
             "Data type for output grid ('int8', 'uint64', "
             "'float', etc.)",
             m_dataType, "double");
    // Nan is a sentinal value to say that no value was set for nodata.
    args.add("nodata", "No data value", m_noData,
             std::numeric_limits<double>::quiet_NaN());
}

void RasterWriter::write(const PointViewPtr view)
{
    m_rustViews.push_back(rust_view_converter::toRust(view));
}

void RasterWriter::done(PointTableRef)
{
    if (m_rustViews.empty())
        return;

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", filename());
    addOption(options, "gdaldriver", m_drivername);
    for (const std::string& option : m_options)
        addOption(options, "gdalopts", option);
    for (const std::string& rasterName : m_rasterNames)
        addOption(options, "rasters", rasterName);
    addOption(options, "data_type", m_dataType);
    addOption(options, "nodata", m_noData);

    pdal_writer_t* writer = pdal_writer_create_raster(options);
    if (!writer)
    {
        pdal_options_destroy(options);
        rust_view_converter::throwLastError(
            "Failed to create Rust raster writer.");
    }

    std::vector<const pdal_point_view_t*> rustViews(m_rustViews.begin(),
                                                    m_rustViews.end());
    bool ok =
        pdal_writer_write_views(writer, rustViews.data(), rustViews.size());
    pdal_writer_destroy(writer);
    pdal_options_destroy(options);
    if (!ok)
        rust_view_converter::throwLastError("Rust raster writer failed.");

    for (pdal_point_view_t* view : m_rustViews)
        pdal_point_view_destroy(view);
    m_rustViews.clear();

    getMetadata().addList("filename", filename());
}

} // namespace pdal
