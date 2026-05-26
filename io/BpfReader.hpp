/******************************************************************************
 * Copyright (c) 2014, Andrew Bell
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

#include <memory>
#include <vector>

#include <pdal/Reader.hpp>
#include <pdal/Streamable.hpp>
#include <pdal/pdal_export.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

namespace pdal
{

class PDAL_EXPORT BpfReader : public Reader, public Streamable
{
    struct Args;

public:
    BpfReader();
    ~BpfReader() override;

    std::string getName() const override;

    virtual point_count_t numPoints() const
    {
        if (m_rustView)
            return (point_count_t)pdal_point_view_length(m_rustView);
        return 0;
    }

private:
    std::unique_ptr<Args> m_args;

    std::string m_remoteFilename;
    pdal_point_view_t* m_rustView = nullptr;
    Dimension::IdList m_rustDims;
    StringList m_rustDimNames;
    PointId m_rustIndex = 0;

    QuickInfo inspect() override;
    void initialize() override;
    void addDimensions(PointLayoutPtr Layout) override;
    void addArgs(ProgramArgs& args) override;
    void ready(PointTableRef table) override;
    bool processOne(PointRef& point) override;
    point_count_t read(PointViewPtr data, point_count_t num) override;
    void done(PointTableRef table) override;

    bool eof();

    void copyRustPoint(PointRef& point, PointId rustIndex);
    void cleanupRemoteFile();
};

} // namespace pdal
