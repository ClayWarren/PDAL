/******************************************************************************
 * Copyright (c) 2015, Hobu Inc., hobu@hobu.co
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

// BPF is an NGA specification for point cloud data. The specification can be
// found at https://nsgreg.nga.mil/doc/view?i=4202

#pragma once

#include "BpfHeader.hpp"

#include <pdal/FlexWriter.hpp>
#include <pdal/pdal_export.hpp>
#include <pdal/util/OStream.hpp>
#include <pdal_capi.h>

#include <vector>

namespace pdal
{

class PDAL_EXPORT BpfWriter : public FlexWriter
{
public:
    struct CoordId
    {
        CoordId() : m_auto(false), m_val(0) {}

        CoordId(bool isAuto, int val) : m_auto(isAuto), m_val(val) {}

        bool m_auto;
        int m_val;
    };

    std::string getName() const override;
    ~BpfWriter() override;

private:
    StringList m_outputDims; ///< List of dimensions to write
    BpfFormat m_format;
    BpfDimensionList m_dims;
    std::vector<Dimension::Type> m_dimTypes;
    bool m_compression;
    CoordId m_coordId;
    std::string m_extraDataSpec;
    StringList m_bundledFilesSpec;
    std::string m_curFilename;
    std::string m_remoteFilename;
    SpatialReference m_curSrs;
    pdal_point_view_t* m_rustView = nullptr;

    void addArgs(ProgramArgs& args) override;
    void initialize() override;
    void prepared(PointTableRef table) override;
    void readyFile(const std::string& filename,
                   const SpatialReference& srs) override;
    void prerunFile(const PointViewSet& pvSet) override;
    void writeView(const PointViewPtr data) override;
    void doneFile() override;

    void loadBpfDimensions(PointLayoutPtr layout);
    void copyViewToRust(const PointViewPtr data);
    void writeRustView();
};

} // namespace pdal
