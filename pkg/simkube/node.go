package simkube

import (
	"fmt"
	"os"

	corev1 "k8s.io/api/core/v1"
	"sigs.k8s.io/yaml"
)

const podNameEnv = "POD_NAME"

func parseSkeletonNode(nodeSkeletonFile string) (*corev1.Node, error) {
	var skel corev1.Node
	nodeBytes, err := os.ReadFile(nodeSkeletonFile)
	if err != nil {
		return nil, fmt.Errorf("could not open %s: %w", nodeSkeletonFile, err)
	}

	if err = yaml.UnmarshalStrict(nodeBytes, &skel); err != nil {
		return nil, fmt.Errorf("could not parse %s: %w", nodeSkeletonFile, err)
	}

	return &skel, nil
}

func makeNode(nodeSkeletonFile string) (*corev1.Node, error) {
	node, err := parseSkeletonNode(nodeSkeletonFile)
	if err != nil {
		return nil, err
	}

	nodeName := os.Getenv(podNameEnv)
	if nodeName == "" {
		return nil, fmt.Errorf("could not determine pod name")
	}

	node.ObjectMeta.Name = nodeName

	return node, nil
}
