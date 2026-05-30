#pragma once

#include <stdexcept>
#include <utility>
#include <vector>

// Opaque handle to the Rust grid-accelerated point-in-polygon engine.
struct pdal_gridpnp;

namespace pdal
{

struct grid_error : public std::runtime_error
{
    grid_error(const std::string& s) : std::runtime_error(s) {}
};

// Thin wrapper over the Rust `pdal_filters::gridpnp::GridPnp` engine, reached
// through the C ABI (`pdal_gridpnp_*`). Build once from a polygon's rings,
// then query `inside` per point. See the Rust implementation for the
// grid/voxel-ray-trace algorithm; this class only marshals rings across the
// ABI and owns the handle.
class GridPnp
{
public:
    using Point = std::pair<double, double>;
    using Ring = std::vector<Point>;

    GridPnp(const Ring& outer, const std::vector<Ring>& inners);
    GridPnp(const Ring& outer);
    ~GridPnp();

    GridPnp(const GridPnp&) = delete;
    GridPnp& operator=(const GridPnp&) = delete;

    bool inside(const Point& p) const
    {
        return inside(p.first, p.second);
    }

    bool inside(double x, double y) const;

private:
    struct pdal_gridpnp* m_handle;
};

} // namespace pdal
