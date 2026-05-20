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

#include <pdal/PointTable.hpp>
#include <pdal/PointView.hpp>
#include <pdal/Reader.hpp>
#include <rust/pdal-capi/include/pdal_capi.h>

#include "OptechCommon.hpp"

namespace pdal
{

class PDAL_EXPORT OptechReader : public Reader
{
public:
    std::string getName() const override;

    static const size_t MaximumNumberOfReturns = 4;
    static const size_t NumBytesInRecord = 69;
    static const size_t BufferSize = 1000000;
    static const size_t MaxNumRecordsInBuffer = BufferSize / NumBytesInRecord;

    OptechReader();
    ~OptechReader() override;

    const CsdHeader& getHeader() const;

private:
    void initialize() override;
    void addDimensions(PointLayoutPtr layout) override;
    void ready(PointTableRef table) override;
    point_count_t read(PointViewPtr view, point_count_t num) override;
    void done(PointTableRef table) override;
    void copyPoint(PointViewPtr view, PointId outIdx);

    CsdHeader m_header;
    pdal_point_view_t* m_rustView = nullptr;
    PointId m_rustIndex = 0;
    Dimension::IdList m_dims;
};
} // namespace pdal
