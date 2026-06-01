/******************************************************************************
 * Copyright (c) 2015, Howard Butler (howard@hobu.co)
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

#pragma once

#include <filters/MergeFilter.hpp>
#include <pdal/PointView.hpp>
#include <pdal/Reader.hpp>
#include <pdal/StageFactory.hpp>
#include <pdal_capi.h>

// Get GDAL's forward decls if available
// otherwise make our own
#if __has_include(<gdal_fwd.h>)
#include <gdal_fwd.h>
#else
using OGRDataSourceH = void*;
using OGRLayerH = void*;
#endif

namespace pdal
{

namespace gdal
{
class SpatialRef;
}

class PDAL_EXPORT TIndexReader : public Reader, public Streamable
{
    struct FileInfo
    {
        std::string m_filename;
        std::string m_srs;
        std::string m_boundary;
        struct tm m_ctime;
        struct tm m_mtime;
    };

    struct FieldIndexes
    {
        int m_filename;
        int m_srs;
        int m_ctime;
        int m_mtime;
    };

public:
    TIndexReader();
    ~TIndexReader() override;

    std::string getName() const override;

private:
    void addDimensions(PointLayoutPtr layout) override;
    void addArgs(ProgramArgs& args) override;
    void initialize() override;
    void prepared(PointTableRef table) override;
    void ready(PointTableRef table) override;
    PointViewSet run(PointViewPtr view) override;
    point_count_t read(PointViewPtr view, point_count_t num) override;
    bool processOne(PointRef& point) override;

    struct Args;
    std::unique_ptr<Args> m_args;

    std::unique_ptr<gdal::SpatialRef> m_out_ref;
    OGRDataSourceH m_dataset;
    OGRLayerH m_layer;

    StageFactory m_factory;
    MergeFilter m_merge;
    pdal_point_view_t* m_rustView = nullptr;
    Dimension::IdList m_rustDims;
    StringList m_rustDimNames;
    PointId m_rustIndex = 0;
    bool m_useRustReader = false;

    std::vector<FileInfo> getFiles();
    FieldIndexes getFields();
    bool canUseRustReader() const;
    void loadRustView();
    void copyRustPoint(PointRef& point, PointId rustIndex);
};

} // namespace pdal
