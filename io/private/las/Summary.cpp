/******************************************************************************
 * Copyright (c) 2011, Michael P. Gerlek (mpg@flaxen.com)
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

#include "Summary.hpp"

#include <pdal_capi.h>

namespace pdal
{
namespace las
{

Summary::Summary()
{
    m_summary = pdal_las_summary_create();
}

Summary::~Summary()
{
    pdal_las_summary_destroy(m_summary);
}

void Summary::clear()
{
    pdal_las_summary_clear(m_summary);
}

void Summary::addPoint(double x, double y, double z, int returnNumber)
{
    pdal_las_summary_add_point(m_summary, x, y, z, returnNumber);
}

point_count_t Summary::getTotalNumPoints() const
{
    return pdal_las_summary_total_num_points(m_summary);
}

BOX3D Summary::getBounds() const
{
    pdal_bounds3d_t bounds;
    pdal_las_summary_bounds(m_summary, &bounds);
    return BOX3D(bounds.minx, bounds.miny, bounds.minz, bounds.maxx,
                 bounds.maxy, bounds.maxz);
}

point_count_t Summary::getReturnCount(int returnNumber) const
{
    return pdal_las_summary_return_count(m_summary, returnNumber);
}

void Summary::dump(std::ostream& str) const
{
    str << getBounds();
    str << "Number of returns:";
    for (int i = 0; i < las::Header::ReturnCount; ++i)
        str << " " << getReturnCount(i);
    str << "\n";

    str << "Total number of points: " << getTotalNumPoints() << "\n";
}

std::ostream& operator<<(std::ostream& ostr, const Summary& data)
{
    data.dump(ostr);
    return ostr;
}

} // namespace las
} // namespace pdal
