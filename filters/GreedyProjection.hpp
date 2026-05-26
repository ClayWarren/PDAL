/*
 * Software License Agreement (BSD License)
 *
 *  Point Cloud Library (PCL) - www.pointclouds.org
 *  Copyright (c) 2010-2011, Willow Garage, Inc.
 *
 *  All rights reserved.
 *
 *  Redistribution and use in source and binary forms, with or without
 *  modification, are permitted provided that the following conditions
 *  are met:
 *
 *   * Redistributions of source code must retain the above copyright
 *     notice, this list of conditions and the following disclaimer.
 *   * Redistributions in binary form must reproduce the above
 *     copyright notice, this list of conditions and the following
 *     disclaimer in the documentation and/or other materials provided
 *     with the distribution.
 *   * Neither the name of Willow Garage, Inc. nor the names of its
 *     contributors may be used to endorse or promote products derived
 *     from this software without specific prior written permission.
 *
 *  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 *  "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 *  LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 *  FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 *  COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 *  INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 *  BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
 *  LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 *  CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 *  LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN
 *  ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 *  POSSIBILITY OF SUCH DAMAGE.
 *
 * $Id$
 *
 */

#pragma once

#include <cmath>
#include <fstream>
#include <iostream>

#include <Eigen/Dense>
#include <pdal/Filter.hpp>

namespace pdal
{
class TriangularMesh;

/** \brief Returns if a point X is visible from point R (or the origin)
 * when taking into account the segment between the points S1 and S2
 * \param X 2D coordinate of the point
 * \param S1 2D coordinate of the segment's first point
 * \param S2 2D coordinate of the segment's secont point
 * \param R 2D coorddinate of the reference point (defaults to 0,0)
 * \ingroup surface
 */
inline bool isVisible(const Eigen::Vector2d& X, const Eigen::Vector2d& S1,
                      const Eigen::Vector2d& S2,
                      const Eigen::Vector2d& R = Eigen::Vector2d::Zero())
{
    double a0 = S1[1] - S2[1];
    double b0 = S2[0] - S1[0];
    double c0 = S1[0] * S2[1] - S2[0] * S1[1];
    double a1 = -X[1];
    double b1 = X[0];
    double c1 = 0;
    if (R != Eigen::Vector2d::Zero())
    {
        a1 += R[1];
        b1 -= R[0];
        c1 = R[0] * X[1] - X[0] * R[1];
    }
    double div = a0 * b1 - b0 * a1;
    double x = (b0 * c1 - b1 * c0) / div;
    double y = (a1 * c0 - a0 * c1) / div;

    bool intersection_outside_XR;
    if (R == Eigen::Vector2d::Zero())
    {
        if (X[0] > 0)
            intersection_outside_XR = (x <= 0) || (x >= X[0]);
        else if (X[0] < 0)
            intersection_outside_XR = (x >= 0) || (x <= X[0]);
        else if (X[1] > 0)
            intersection_outside_XR = (y <= 0) || (y >= X[1]);
        else if (X[1] < 0)
            intersection_outside_XR = (y >= 0) || (y <= X[1]);
        else
            intersection_outside_XR = true;
    }
    else
    {
        if (X[0] > R[0])
            intersection_outside_XR = (x <= R[0]) || (x >= X[0]);
        else if (X[0] < R[0])
            intersection_outside_XR = (x >= R[0]) || (x <= X[0]);
        else if (X[1] > R[1])
            intersection_outside_XR = (y <= R[1]) || (y >= X[1]);
        else if (X[1] < R[1])
            intersection_outside_XR = (y >= R[1]) || (y <= X[1]);
        else
            intersection_outside_XR = true;
    }
    if (intersection_outside_XR)
        return true;
    else
    {
        if (S1[0] > S2[0])
            return (x <= S2[0]) || (x >= S1[0]);
        else if (S1[0] < S2[0])
            return (x >= S2[0]) || (x <= S1[0]);
        else if (S1[1] > S2[1])
            return (y <= S2[1]) || (y >= S1[1]);
        else if (S1[1] < S2[1])
            return (y >= S2[1]) || (y <= S1[1]);
        else
            return false;
    }
}

/** \brief GreedyProjectionTriangulation is an implementation of a greedy
 * triangulation algorithm for 3D points based on local 2D projections. It
 * assumes locally smooth surfaces and relatively smooth transitions between
 * areas with different point densities.
 * \author Zoltan Csaba Marton
 * \ingroup surface
 */
class PDAL_EXPORT GreedyProjection : public Filter
{
public:
    enum class GP3Type
    {
        NONE = -1, // not-defined
        FREE = 0,
        FRINGE = 1,
        BOUNDARY = 2,
        COMPLETED = 3
    };

    /** \brief Empty constructor. */
    GreedyProjection()
        : mu_(0), search_radius_(0),            // must be set by user
          nnn_(100), minimum_angle_(M_PI / 18), // 10 degrees
          maximum_angle_(2 * M_PI / 3),         // 120 degrees
          eps_angle_(M_PI / 4),                 // 45 degrees,
          consistent_(false), consistent_ordering_(false),
          mesh_(nullptr) {};

    std::string getName() const override;

    /** \brief Don't consider points for triangulation if their normal deviates
     * more than this value from the query point's normal.
     * \param[in] eps_angle maximum surface angle
     * \note As normal estimation methods usually give smooth transitions at
     * sharp edges, this ensures correct triangulation by avoiding connecting
     * points from one side to points from the other through forcing the use of
     * the edge points.
     */
    inline void setMaximumSurfaceAngle(double eps_angle)
    {
        eps_angle_ = eps_angle;
    }

    /** \brief Get the maximum surface angle. */
    inline double getMaximumSurfaceAngle() const
    {
        return (eps_angle_);
    }

    /** \brief Set the flag if the input normals are oriented consistently.
      * \param[in] consistent set it to true if the normals are
         consistently oriented
      */
    inline void setNormalConsistency(bool consistent)
    {
        consistent_ = consistent;
    }

    /** \brief Get the flag for consistently oriented normals. */
    inline bool getNormalConsistency() const
    {
        return (consistent_);
    }

    /** \brief Set the flag to order the resulting triangle vertices
        consistently (positive direction around normal).
      * @note Assumes consistently oriented normals (towards the viewpoint)
        -- see setNormalConsistency ()
      * \param[in] consistent_ordering set it to true if triangle
        vertices should be ordered consistently
      */
    inline void setConsistentVertexOrdering(bool consistent_ordering)
    {
        consistent_ordering_ = consistent_ordering;
    }

    /** \brief Get the flag signaling consistently ordered triangle vertices. */
    inline bool getConsistentVertexOrdering() const
    {
        return (consistent_ordering_);
    }

    /** \brief Get the state of each point after reconstruction.
     * \note Options are defined as constants: FREE, FRINGE, COMPLETED, BOUNDARY
     * and NONE
     */

protected:
    /** \brief The nearest neighbor distance multiplier to obtain the
        final search radius.
    */
    double mu_;

    /** \brief Nearest neighbors search radius for each point and
        the maximum edge length.
    */
    double search_radius_;

    /** \brief  Maximum number of nearest neighbors accepted by searching. */
    int nnn_;

    /** \brief The preferred minimum angle for the triangles. */
    double minimum_angle_;

    /** \brief The maximum angle for the triangles. */
    double maximum_angle_;

    /** \brief Maximum surface angle. */
    double eps_angle_;

    /** \brief Set this to true if the normals of the input are
        consistently oriented.
    */
    bool consistent_;

    /** \brief Set this to true if the output triangle vertices should
        be consistently oriented.
    */
    bool consistent_ordering_;

private:
    TriangularMesh* mesh_;

    void addArgs(ProgramArgs& args) override;
    void addDimensions(PointLayoutPtr layout) override;
    void initialize() override;
    void filter(PointView& view) override;
};

} // namespace pdal
