#pragma once

#include <pdal/Metadata.hpp>
#include <pdal/pdal_types.hpp>
#include <pdal_capi.h>

#include <cstdint>
#include <string>

namespace pdal
{
namespace rust_metadata
{

inline std::string takeString(char* value)
{
    if (!value)
        return std::string();
    std::string result(value);
    pdal_string_free(value);
    return result;
}

inline void addTo(MetadataNode parent, const pdal_metadata_node_t* rustNode)
{
    if (!rustNode)
        return;

    std::string name = takeString(pdal_metadata_node_name(rustNode));
    MetadataNode node;
    switch (pdal_metadata_node_value_kind(rustNode))
    {
    case 0:
        node = parent.add(name, takeString(pdal_metadata_node_value(rustNode)));
        break;
    case 1:
        node = parent.add(name, pdal_metadata_node_value_i64(rustNode));
        break;
    case 2:
        node = parent.add(
            name, static_cast<point_count_t>(
                      pdal_metadata_node_value_u64(rustNode)));
        break;
    case 3:
        node = parent.add(name, pdal_metadata_node_value_f64(rustNode));
        break;
    case 4:
        node = parent.add(name, pdal_metadata_node_value_bool(rustNode));
        break;
    default:
        node = parent.add(name);
        break;
    }

    uint64_t childCount = pdal_metadata_node_child_count(rustNode);
    for (uint64_t idx = 0; idx < childCount; ++idx)
    {
        pdal_metadata_node_t* child = pdal_metadata_node_child(rustNode, idx);
        addTo(node, child);
        pdal_metadata_node_destroy(child);
    }
}

inline void addChildrenTo(MetadataNode parent, const pdal_metadata_node_t* root)
{
    if (!root)
        return;

    uint64_t childCount = pdal_metadata_node_child_count(root);
    for (uint64_t idx = 0; idx < childCount; ++idx)
    {
        pdal_metadata_node_t* child = pdal_metadata_node_child(root, idx);
        addTo(parent, child);
        pdal_metadata_node_destroy(child);
    }
}

} // namespace rust_metadata
} // namespace pdal
